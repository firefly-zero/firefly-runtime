use super::ring::RingBuf;
use super::*;
use crate::config::FullID;
use alloc::boxed::Box;
use firefly_hal::*;

const SYNC_EVERY: Duration = Duration::from_ms(20);
const FRAME_TIMEOUT: Duration = Duration::from_ms(5000);
const MAX_PEERS: usize = 8;
const MSG_SIZE: usize = 64;

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

pub(crate) struct FrameSyncer<'a> {
    pub frame: u32,
    pub peers: heapless::Vec<FSPeer, MAX_PEERS>,
    pub initial_seed: u32,
    pub app: FullID,
    pub(super) last_sync: Option<Instant>,
    pub(super) last_advance: Option<Instant>,
    pub(super) net: NetworkImpl<'a>,
}

impl<'a> FrameSyncer<'a> {
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

    /// Convert [`FrameSyncer`] back into [`Connection`].
    ///
    /// Used when the game exits back into launcher
    /// so that players can launch another app.
    pub fn into_connection(self) -> Connection<'a> {
        let mut peers = heapless::Vec::<Peer, 8>::new();
        for peer in self.peers {
            let peer = Peer {
                addr: peer.addr,
                name: peer.name,
                intro: None,
            };
            peers.push(peer).ok().unwrap();
        }
        Connection {
            app: None,
            seed: None,
            peers,
            net: self.net,
            last_sync: None,
            last_ready: None,
            started_at: None,
        }
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

    /// Get the combined random seed of all peers.
    pub fn get_seed(&self) -> u32 {
        let mut seed = 0;
        for peer in &self.peers {
            let state = peer.states.get_current();
            if let Some(state) = state {
                seed ^= state.rand;
            };
        }
        if seed == 0 {
            seed = 1;
        }
        seed
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

    pub fn get_action(&mut self) -> Action {
        let mut action = Action::None;
        for peer in &self.peers {
            let Some(state) = peer.states.get_current() else {
                // We don't do the action until all peers are ready.
                return Action::None;
            };
            if state.action != Action::None {
                action = state.action;
            }
        }
        action
    }

    /// Go to the next frame and set that frame's state.
    ///
    /// It will also broadcast the new frame state to all connected peers.
    pub fn advance(&mut self, device: &mut DeviceImpl, mut state: FrameState) {
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
                // Random jitter to avoid all devices transmitting at the same time.
                let delay_us = device.random() % 4;
                device.delay(Duration::from_us(delay_us));
                let res = self.net.send(addr, raw);
                if let Err(err) = res {
                    device.log_error("netcode", err);
                }
            }
        }
        let now = device.now();
        self.last_sync = Some(now);
    }

    fn update_inner(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
        self.sync(device)?;
        while let Some((addr, msg)) = self.net.recv()? {
            self.handle_message(addr, msg)?;
        }
        Ok(())
    }

    /// Get every connected peer with unknown state for the current frame
    /// and send them a request for that state.
    fn sync(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
        let now = device.now();
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

    fn handle_message(&mut self, addr: Addr, raw: Box<[u8]>) -> Result<(), NetcodeError> {
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
        match req {
            Req::State(frame) => self.handle_state_req(addr, frame)?,
            Req::Start => self.handle_start_req(addr)?,
            _ => return Err(NetcodeError::UnexpectedRequest),
        }
        Ok(())
    }

    fn handle_start_req(&mut self, addr: Addr) -> Result<(), NetcodeError> {
        let me = self.get_me();
        let resp = Start {
            id: self.app.clone(),
            badges: me.badges.clone(),
            scores: me.scores.clone(),
            stash: me.stash.clone().into_boxed_slice(),
            seed: self.initial_seed,
        };
        let resp = Message::Resp(Resp::Start(resp));
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = resp.encode(&mut buf)?;
        self.net.send(addr, raw)?;
        Ok(())
    }

    fn handle_state_req(&mut self, addr: Addr, frame: u32) -> Result<(), NetcodeError> {
        let me = self.get_me();
        let state = me.states.get(frame);
        if let Some(state) = state {
            let msg = Message::Resp(Resp::State(state));
            let mut buf = alloc::vec![0u8; MSG_SIZE];
            let raw = msg.encode(&mut buf)?;
            self.net.send(addr, raw)?;
        };
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
