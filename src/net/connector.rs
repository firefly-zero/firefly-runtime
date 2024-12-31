use super::*;
use alloc::boxed::Box;
use firefly_hal::*;

const ADVERTISE_EVERY: Duration = Duration::from_ms(100);
const MAX_PEERS: usize = 7;
const MSG_SIZE: usize = 64;

pub(crate) struct MyInfo {
    pub name: heapless::String<16>,
    pub version: u16,
}

pub(crate) struct PeerInfo {
    pub addr: Addr,
    pub name: heapless::String<16>,
    // TODO: check that the version is compatible
    // pub version: u16,
}

/// Connector establishes network connection between devices.
///
/// If you play games with friends, you establish the connection once
/// at the beginning of the evening and it stays on as long as
/// all the devices are turned on.
pub(crate) struct Connector<'a> {
    pub me: MyInfo,
    net: NetworkImpl<'a>,
    last_advertisement: Option<Instant>,
    peer_addrs: heapless::Vec<Addr, MAX_PEERS>,
    peer_infos: heapless::Vec<PeerInfo, MAX_PEERS>,
    started: bool,
    stopped: bool,
}

impl<'a> Connector<'a> {
    pub fn new(me: MyInfo, net: NetworkImpl<'a>) -> Self {
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

    pub fn finalize(self) -> Connection<'a> {
        let mut peers = heapless::Vec::<Peer, 8>::new();
        for peer in self.peer_infos {
            let peer = Peer {
                addr: Some(peer.addr),
                name: peer.name,
                intro: None,
            };
            peers.push(peer).ok().unwrap();
        }
        let me = Peer {
            addr: None,
            name: self.me.name,
            intro: None,
        };
        peers.push(me).ok().unwrap();
        let local_addr = self.net.local_addr();
        peers.sort_by_key(|p| p.addr.unwrap_or(local_addr));
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
        raw: Box<[u8]>,
    ) -> Result<(), NetcodeError> {
        if !self.peer_addrs.contains(&addr) {
            device.log_debug("netcode", "new device discovered");
            let res = self.peer_addrs.push(addr);
            if res.is_err() {
                return Err(NetcodeError::PeerListFull);
            }
            self.send_intro(device, addr)?;
        }
        let msg = Message::decode(&raw)?;
        match msg {
            Message::Req(req) => self.handle_req(device, addr, req),
            Message::Resp(resp) => self.handle_resp(addr, resp),
        }
    }

    fn handle_req(
        &mut self,
        device: &DeviceImpl,
        addr: Addr,
        req: Req,
    ) -> Result<(), NetcodeError> {
        if matches!(req, Req::Intro) {
            self.send_intro(device, addr)?
        }
        Ok(())
    }

    fn handle_resp(&mut self, addr: Addr, resp: Resp) -> Result<(), NetcodeError> {
        if let Resp::Intro(intro) = resp {
            self.handle_intro(addr, intro)?;
        }
        Ok(())
    }

    fn handle_intro(&mut self, addr: Addr, intro: Intro) -> Result<(), NetcodeError> {
        for info in &self.peer_infos {
            if info.addr == addr {
                return Ok(());
            }
        }
        // TODO: validate the name
        let info = PeerInfo {
            addr,
            name: intro.name,
            // version: intro.version,
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
        let raw = msg.encode(&mut buf)?;
        self.net.send(addr, raw)?;
        Ok(())
    }
}
