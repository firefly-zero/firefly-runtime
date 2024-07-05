use super::*;
use firefly_device::*;

const ADVERTISE_EVERY: Duration = Duration::from_ms(100);
const MAX_PEERS: usize = 7;
const MSG_SIZE: usize = 64;
type Addr = <NetworkImpl as Network>::Addr;

pub(crate) struct MyInfo {
    pub name: heapless::String<16>,
    pub version: u16,
}

pub(crate) struct PeerInfo {
    pub addr: Addr,
    pub name: heapless::String<16>,
    pub version: u16,
}

/// Connector establishes network connection between devices.
///
/// If you play games with friends, you establish the connection once
/// at the beginning of the evening and it stays on as long as
/// all the devices are turned on.
pub(crate) struct Connector {
    pub me: MyInfo,
    net: NetworkImpl,
    last_advertisement: Option<Instant>,
    peer_addrs: heapless::Vec<Addr, MAX_PEERS>,
    peer_infos: heapless::Vec<PeerInfo, MAX_PEERS>,
    started: bool,
    stopped: bool,
}

impl Connector {
    pub fn new(me: MyInfo, net: NetworkImpl) -> Self {
        Self {
            me,
            net,
            last_advertisement: None,
            peer_addrs: heapless::Vec::new(),
            peer_infos: heapless::Vec::new(),
            started: false,
            stopped: false,
        }
    }

    pub fn peer_addrs(&self) -> &heapless::Vec<Addr, MAX_PEERS> {
        &self.peer_addrs
    }

    pub fn peer_infos(&self) -> &heapless::Vec<PeerInfo, MAX_PEERS> {
        &self.peer_infos
    }

    pub fn update(&mut self, device: &DeviceImpl) {
        let res = self.update_inner(device);
        if let Err(err) = res {
            device.log_error("netcode", err);
        }
    }

    /// Stop announcing and accepting new connections.
    pub fn pause(&mut self) -> Result<(), NetcodeError> {
        self.stopped = true;
        Ok(())
    }

    /// Stop all network operations.
    pub fn cancel(&mut self) -> Result<(), NetcodeError> {
        self.stopped = true;
        self.net.stop()?;
        Ok(())
    }

    pub fn finalize(self) -> Connection {
        let mut peers = heapless::Vec::<Peer, 8>::new();
        for peer in self.peer_infos {
            let peer = Peer {
                addr: Some(peer.addr),
                name: peer.name,
                ready: false,
            };
            peers.push(peer).ok().unwrap();
        }
        let me = Peer {
            addr: None,
            name: self.me.name,
            ready: false,
        };
        peers.push(me).ok().unwrap();
        Connection {
            peers,
            app: None,
            net: self.net,
            last_sync: None,
            last_ready: None,
        }
    }

    fn update_inner(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
        if self.stopped {
            return Ok(());
        }
        if !self.started {
            self.started = true;
            self.net.start()?;
        }
        let now = device.now();
        self.advertise(now)?;
        if let Some((addr, msg)) = self.net.recv()? {
            self.handle_message(device, addr, msg)?;
        }
        Ok(())
    }

    fn advertise(&mut self, now: Instant) -> Result<(), NetcodeError> {
        if let Some(prev) = self.last_advertisement {
            if now - prev < ADVERTISE_EVERY {
                return Ok(());
            }
        }
        self.last_advertisement = Some(now);
        self.net.advertise()?;
        Ok(())
    }

    fn handle_message(
        &mut self,
        device: &DeviceImpl,
        addr: Addr,
        raw: heapless::Vec<u8, MSG_SIZE>,
    ) -> Result<(), NetcodeError> {
        if !self.peer_addrs.contains(&addr) {
            device.log_debug("netcode", "new device discovered");
            let res = self.peer_addrs.push(addr);
            if res.is_err() {
                return Err(NetcodeError::PeerListFull);
            }
            self.send_intro(device, addr)?;
        }
        if raw == b"HELLO" {
            return Ok(());
        }
        if raw.is_empty() {
            return Err(NetcodeError::EmptyBufferIn);
        }
        let msg = match Message::decode(&raw) {
            Ok(msg) => msg,
            Err(err) => return Err(NetcodeError::Deserialize(err)),
        };
        match msg {
            Message::Req(req) => self.handle_req(device, addr, req),
            Message::Resp(resp) => self.handle_resp(device, addr, resp),
        }
    }

    fn handle_req(
        &mut self,
        device: &DeviceImpl,
        addr: Addr,
        req: Req,
    ) -> Result<(), NetcodeError> {
        match req {
            Req::Intro => self.send_intro(device, addr),
            _ => Ok(()),
        }
    }

    fn handle_resp(
        &mut self,
        device: &DeviceImpl,
        addr: Addr,
        resp: Resp,
    ) -> Result<(), NetcodeError> {
        match resp {
            Resp::Intro(intro) => self.handle_intro(device, addr, intro),
            _ => Ok(()),
        }
    }

    fn handle_intro(
        &mut self,
        device: &DeviceImpl,
        addr: Addr,
        intro: Intro,
    ) -> Result<(), NetcodeError> {
        for info in &self.peer_infos {
            if info.addr == addr {
                return Ok(());
            }
        }
        // TODO: validate the name
        device.log_debug("netcode", &intro.name);
        let info = PeerInfo {
            addr,
            name: intro.name,
            version: intro.version,
        };
        let res = self.peer_infos.push(info);
        if res.is_err() {
            return Err(NetcodeError::PeerListFull);
        }
        Ok(())
    }

    fn send_intro(&mut self, _device: &DeviceImpl, addr: Addr) -> Result<(), NetcodeError> {
        let intro = Intro {
            name: self.me.name.clone(),
            version: self.me.version,
        };
        let msg = Message::Resp(intro.into());
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = match msg.encode(&mut buf) {
            Ok(raw) => raw,
            Err(err) => return Err(NetcodeError::Serialize(err)),
        };
        if raw.is_empty() {
            return Err(NetcodeError::EmptyBufferOut);
        }
        self.net.send(addr, raw)?;
        Ok(())
    }
}
