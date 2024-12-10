use super::ring::RingBuf;
use super::*;
use alloc::boxed::Box;
use firefly_hal::*;

const SYNC_EVERY: Duration = Duration::from_ms(5);
const FRAME_TIMEOUT: Duration = Duration::from_ms(100);
const MAX_PEERS: usize = 8;
const MSG_SIZE: usize = 64;
type Addr = <NetworkImpl as Network>::Addr;

pub(crate) struct FSPeer {
    /// If address is None, the peer is the current device.
    pub addr: Option<Addr>,
    /// The human-readable name of the device.
    pub name: heapless::String<16>,
    /// The peer's index in /sys/friends.
    pub friend_id: Option<u16>,
    pub states: RingBuf<FrameState>,
    /// The peer's progress for each badge.
    pub badges: Box<[u16]>,
    /// The peer's top score for each board.
    pub scores: Box<[i16]>,
    /// The peer's shared state (save files, character, inventory, etc).
    pub stash: alloc::vec::Vec<u8>,
}

pub(crate) struct FrameSyncer {
    pub frame: u32,
    pub peers: heapless::Vec<FSPeer, MAX_PEERS>,
    pub(super) last_sync: Option<Instant>,
    pub(super) last_advance: Option<Instant>,
    pub(super) net: NetworkImpl,
}

impl FrameSyncer {
    /// Check if we have the state of the current frame for all connected peers.
    pub fn ready(&self) -> bool {
        for peer in &self.peers {
            let state = peer.states.get_current();
            if state.is_none() {
                return false;
            }
        }
        true
    }

    /// Get combined input of all peers.
    ///
    /// A button is considered pressed if any peer presses it.
    pub fn get_combined_input(&self) -> InputState {
        let mut input = InputState::default();
        for peer in &self.peers {
            let state = peer.states.get_current();
            if let Some(state) = state {
                input = input.merge(&state.input.into());
            };
        }
        input
    }

    pub fn update(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
        let now = device.now();
        if now - self.last_advance.unwrap() > FRAME_TIMEOUT {
            return Err(NetcodeError::FrameTimeout);
        }
        let res = self.update_inner(device);
        if let Err(err) = res {
            device.log_error("netcode", err);
        }
        Ok(())
    }

    /// Go to the next frame and set that frame's state.
    ///
    /// It will also broadcast the new frame state to all connected peers.
    pub fn advance(&mut self, device: &DeviceImpl, mut state: FrameState) {
        self.frame += 1;

        // Set the frame state for the local peer.
        state.frame = self.frame;
        for peer in &mut self.peers {
            peer.states.advance();
            if peer.addr.is_none() {
                peer.states.insert_current(state);
            }
        }
        self.last_advance = Some(device.now());

        // Send the new frame state to all peers.
        let msg = Message::Resp(Resp::State(state));
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = match msg.encode(&mut buf) {
            Ok(raw) => raw,
            Err(err) => {
                device.log_error("netcode", err);
                return;
            }
        };
        for peer in &mut self.peers {
            if let Some(addr) = peer.addr {
                let res = self.net.send(addr, raw);
                if let Err(err) = res {
                    device.log_error("netcode", err);
                }
            }
        }
    }

    fn update_inner(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
        let now = device.now();
        self.sync(now)?;
        if let Some((addr, msg)) = self.net.recv()? {
            self.handle_message(addr, msg)?;
        }
        Ok(())
    }

    /// Get every connected peer with unknown state for the current frame
    /// and send them a request for that state.
    fn sync(&mut self, now: Instant) -> Result<(), NetcodeError> {
        if let Some(prev) = self.last_sync {
            if now - prev < SYNC_EVERY {
                return Ok(());
            }
        }
        self.last_sync = Some(now);
        let msg = Message::Req(Req::State(self.frame));
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = msg.encode(&mut buf)?;
        for peer in &self.peers {
            let Some(addr) = peer.addr else {
                continue;
            };
            let state = peer.states.get_current();
            if state.is_none() {
                self.net.send(addr, raw)?;
            }
        }
        Ok(())
    }

    fn handle_message(
        &mut self,
        addr: Addr,
        raw: heapless::Vec<u8, MSG_SIZE>,
    ) -> Result<(), NetcodeError> {
        if !self.peers.iter().any(|p| p.addr == Some(addr)) {
            return Err(NetcodeError::UnknownPeer);
        }
        let msg = Message::decode(&raw)?;
        match msg {
            Message::Req(req) => self.handle_req(addr, req),
            Message::Resp(resp) => self.handle_resp(addr, resp),
        }
    }

    fn handle_req(&mut self, addr: Addr, req: Req) -> Result<(), NetcodeError> {
        // A peer requested a state for a specific frame.
        // Send them the state if available.
        // If not, send nothing, let them timeout.
        if let Req::State(frame) = req {
            let me = self.get_me();
            let state = me.states.get(frame);
            if let Some(state) = state {
                let msg = Message::Resp(Resp::State(state));
                let mut buf = alloc::vec![0u8; MSG_SIZE];
                let raw = msg.encode(&mut buf)?;
                self.net.send(addr, raw)?;
            }
        }
        Ok(())
    }

    fn handle_resp(&mut self, addr: Addr, resp: Resp) -> Result<(), NetcodeError> {
        // A peer reported their state for a frame.
        // Store it in the ring of states.
        if let Resp::State(state) = resp {
            for peer in self.peers.iter_mut() {
                if peer.addr == Some(addr) {
                    peer.states.insert(state.frame, state);
                }
            }
        }
        Ok(())
    }

    /// Get a reference to the peer representing the local device.
    ///
    /// There must be exactly one such peer in the list of peers.
    fn get_me(&self) -> &FSPeer {
        for peer in &self.peers {
            if peer.addr.is_none() {
                return peer;
            }
        }
        unreachable!("the list of peers doesn't have the local device")
    }
}
