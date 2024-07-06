use super::ring::RingBuf;
use super::*;
use firefly_device::*;

const SYNC_EVERY: Duration = Duration::from_ms(1);
const MAX_PEERS: usize = 8;
const MSG_SIZE: usize = 64;
type Addr = <NetworkImpl as Network>::Addr;

pub(crate) struct FSPeer {
    /// If address is None, the peer is the current device.
    pub addr: Option<Addr>,
    pub name: heapless::String<16>,
    pub states: RingBuf<FrameState>,
}

pub(crate) struct FrameSyncer {
    pub frame: u32,
    pub peers: heapless::Vec<FSPeer, MAX_PEERS>,
    pub(super) last_sync: Option<Instant>,
    pub(super) net: NetworkImpl,
}

impl FrameSyncer {
    pub fn ready(&self) -> bool {
        for peer in &self.peers {
            let state = peer.states.get(self.frame);
            if state.is_none() {
                return false;
            }
        }
        true
    }

    pub fn update(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
        let now = device.now();
        self.sync(now)?;
        if let Some((addr, msg)) = self.net.recv()? {
            self.handle_message(addr, msg)?;
        }
        Ok(())
    }

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
            let state = peer.states.get(self.frame);
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
        match resp {
            Resp::State(state) => {
                for peer in self.peers.iter_mut() {
                    if peer.addr == Some(addr) {
                        peer.states.insert(state.frame, state);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn get_me(&self) -> &FSPeer {
        for peer in &self.peers {
            if peer.addr.is_none() {
                return peer;
            }
        }
        unreachable!("the list of peers doesn't have the local device")
    }
}
