use super::ring::RingBuf;
use super::*;
use crate::config::FullID;
use crate::state::log_net_error;
use alloc::boxed::Box;
use firefly_hal::*;

const SYNC_EVERY: Duration = Duration::from_s(2);
const FRAME_TIMEOUT: Duration = Duration::from_s(5);
const FIRST_TIMEOUT: Duration = Duration::from_s(10);
const MAX_PEERS: usize = 8;
const MSG_SIZE: usize = 64;

pub(crate) struct FSPeer {
    /// If address is None, the peer is the current device.
    pub addr: Option<Addr>,
    pub intro: Intro,
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
    /// The initial seed of the current device.
    pub device_seed: u32,
    /// The combined initial seed of all devices.
    pub shared_seed: u32,
    pub app: FullID,
    pub(super) last_sync: Option<Instant>,
    pub(super) last_advance: Option<Instant>,
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

    /// Convert [`FrameSyncer`] back into [`Connection`].
    ///
    /// Used when the game exits back into launcher
    /// so that players can launch another app.
    pub fn into_connection(self) -> Box<Connection> {
        let mut peers = heapless::Vec::<Peer, 8>::new();
        for peer in self.peers {
            let peer = Peer {
                addr: peer.addr,
                intro: peer.intro,
                app: None,
            };
            peers.push(peer).ok().unwrap();
        }
        Box::new(Connection {
            app: None,
            seed: None,
            peers,
            last_sync: None,
            last_ready: None,
            started_at: None,
        })
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
    ///
    /// Returns 0 if RNG was not synced on this frame.
    pub fn get_seed(&self) -> u32 {
        let mut seed = 0;
        for peer in &self.peers {
            let state = peer.states.get_current();
            if let Some(state) = state
                && let Extra::Rand(rand) = state.extra
            {
                seed ^= rand;
            };
        }
        seed
    }

    pub fn get_now(&self) -> Option<u32> {
        let mut min = u32::MAX;
        for peer in &self.peers {
            let state = peer.states.get_current();
            if let Some(state) = state
                && let Extra::Now(now) = state.extra
            {
                min = min.min(now);
            };
        }
        if min == u32::MAX { None } else { Some(min) }
    }

    pub fn update(&mut self, device: &mut DeviceImpl) -> Result<(), NetcodeError> {
        let now = device.now();
        let timeout = if self.frame <= 2 {
            FIRST_TIMEOUT
        } else {
            FRAME_TIMEOUT
        };
        if now - self.last_advance.unwrap() > timeout {
            return Err(NetcodeError::FrameTimeout);
        }
        let res = self.update_inner(device);
        if let Err(err) = res {
            log_net_error(device, err);
        }
        Ok(())
    }

    /// Go to the next frame and set that frame's state.
    ///
    /// It will also broadcast the new frame state to all connected peers.
    pub fn advance(&mut self, device: &mut DeviceImpl, mut state: FrameState) {
        self.frame += 1;
        for peer in &mut self.peers {
            peer.states.advance();
        }

        // This code is responsible for setting the passed state
        // for the next frame instead of the current one.
        if self.frame == 1 {
            state.frame = 1;
            self.set_my_state(state);
            self.broadcast_state(device, state);
        }
        state.frame = self.frame + 1;

        self.set_my_state(state);
        self.broadcast_state(device, state);
        self.last_advance = Some(device.now());
    }

    fn set_my_state(&mut self, state: FrameState) {
        for peer in &mut self.peers {
            if peer.addr.is_none() {
                peer.states.insert(state.frame, state);
                return;
            }
        }
    }

    fn broadcast_state(&mut self, device: &mut DeviceImpl, state: FrameState) {
        let msg = Message::State(state);
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = match msg.encode(&mut buf) {
            Ok(raw) => raw,
            Err(err) => {
                log_net_error(device, err);
                return;
            }
        };

        for peer in &mut self.peers {
            if let Some(addr) = peer.addr {
                let res = device.net_send(addr, raw);
                if let Err(err) = res {
                    log_net_error(device, err);
                }
            }
        }
        let now = device.now();
        self.last_sync = Some(now);
    }

    fn update_inner(&mut self, device: &mut DeviceImpl) -> Result<(), NetcodeError> {
        for _ in 0..4 {
            let Some((addr, msg)) = device.net_recv()? else {
                break;
            };
            self.handle_message(device, addr, msg)?;
        }
        self.sync(device)
    }

    /// Get every connected peer with unknown state for the current frame
    /// and send them a request for that state.
    fn sync(&mut self, device: &mut DeviceImpl) -> Result<(), NetcodeError> {
        let now = device.now();
        if let Some(prev) = self.last_sync
            && now - prev < SYNC_EVERY
        {
            return Ok(());
        }
        device.log_debug("netcode", "requesting sync");
        self.last_sync = Some(now);
        let msg = Message::ReqState(self.frame);
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = msg.encode(&mut buf)?;
        for peer in &self.peers {
            let Some(addr) = peer.addr else {
                continue;
            };
            let state = peer.states.get_current();
            if state.is_none() {
                device.net_send(addr, raw)?;
            }
        }
        Ok(())
    }

    fn handle_message(
        &mut self,
        device: &mut DeviceImpl,
        addr: Addr,
        raw: Box<[u8]>,
    ) -> Result<(), NetcodeError> {
        if !self.peers.iter().any(|p| p.addr == Some(addr)) {
            return Err(NetcodeError::UnknownPeer);
        }
        let msg = Message::decode(&raw)?;
        match msg {
            // A peer requested a state for a specific frame.
            // Send them the state if available.
            // If not, send nothing, let them timeout.
            Message::ReqState(frame) => self.handle_state_req(device, addr, frame)?,
            Message::ReqStart => self.handle_start_req(device, addr)?,
            // A peer reported their state for a frame.
            // Store it in the ring of states.
            Message::State(state) => {
                for peer in self.peers.iter_mut() {
                    if peer.addr == Some(addr) {
                        peer.states.insert(state.frame, state);
                    }
                }
            }
            _ => return Err(NetcodeError::UnexpectedRequest),
        }
        Ok(())
    }

    fn handle_start_req(&self, device: &mut DeviceImpl, addr: Addr) -> Result<(), NetcodeError> {
        let me = self.get_me();
        let resp = Start {
            id: self.app.clone(),
            badges: me.badges.clone(),
            scores: me.scores.clone(),
            stash: me.stash.clone().into_boxed_slice(),
            seed: self.device_seed,
        };
        let resp = Message::Start(resp);
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = resp.encode(&mut buf)?;
        device.net_send(addr, raw)?;
        Ok(())
    }

    fn handle_state_req(
        &self,
        device: &mut DeviceImpl,
        addr: Addr,
        frame: u32,
    ) -> Result<(), NetcodeError> {
        let me = self.get_me();
        let state = me.states.get(frame);
        if let Some(state) = state {
            let msg = Message::State(state);
            let mut buf = alloc::vec![0u8; MSG_SIZE];
            let raw = msg.encode(&mut buf)?;
            device.net_send(addr, raw)?;
        };
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
