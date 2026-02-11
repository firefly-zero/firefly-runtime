use super::*;
use alloc::boxed::Box;
use firefly_hal::*;

const ADVERTISE_EVERY: Duration = Duration::from_ms(100);
const MAX_PEERS: usize = 7;
const MSG_SIZE: usize = 64;

#[derive(PartialEq, Copy, Clone)]
pub(crate) enum ConnectStatus {
    /// Stopped listening, [Connector] should do nothing.
    Stopped,
    /// Cancelled connecting, destroy [Connector].
    Cancelled,
    /// Finished connecting, proceed to multiplayer.
    Finished,
}

pub(crate) struct PeerInfo {
    pub addr: Addr,
    pub intro: Intro,
}

/// Connector establishes network connection between devices.
///
/// If you play games with friends, you establish the connection once
/// at the beginning of the evening and it stays on as long as
/// all the devices are turned on.
pub(crate) struct Connector<'a> {
    pub me: Intro,
    net: NetworkImpl<'a>,
    last_advertisement: Option<Instant>,
    peer_addrs: heapless::Vec<Addr, MAX_PEERS>,
    peer_infos: heapless::Vec<PeerInfo, MAX_PEERS>,
    /// If the network interface (WiFi) has been activated.
    started: bool,
    /// If the device should not accept eny new connections.
    stopped: bool,
    pub status: Option<ConnectStatus>,
}

impl<'a> Connector<'a> {
    pub fn new(me: Intro, net: NetworkImpl<'a>) -> Self {
        Self {
            me,
            net,
            last_advertisement: None,
            peer_addrs: heapless::Vec::new(),
            peer_infos: heapless::Vec::new(),
            started: false,
            stopped: false,
            status: None,
        }
    }

    pub fn peer_addrs(&self) -> &heapless::Vec<Addr, MAX_PEERS> {
        &self.peer_addrs
    }

    pub fn peer_infos(&self) -> &heapless::Vec<PeerInfo, MAX_PEERS> {
        &self.peer_infos
    }

    /// Stop announcing and accepting new connections.
    pub fn pause(&mut self) -> Result<(), NetcodeError> {
        self.stopped = true;
        Ok(())
    }

    /// Stop all network operations.
    pub fn cancel(&mut self) -> Result<(), NetcodeError> {
        self.stopped = true;
        self.send_disconnect()?;
        self.net.stop()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        for peer in &self.peer_infos {
            if peer.intro.version != self.me.version {
                return Err("devices have incompatible OS versions; please, update.");
            }
        }
        Ok(())
    }

    pub fn finalize(self) -> Box<Connection<'a>> {
        let mut peers = heapless::Vec::<Peer, 8>::new();
        for peer in self.peer_infos {
            let peer = Peer {
                addr: Some(peer.addr),
                name: peer.intro.name,
                app: None,
            };
            peers.push(peer).ok().unwrap();
        }
        let me = Peer {
            addr: None,
            name: self.me.name,
            app: None,
        };
        peers.push(me).ok().unwrap();
        let local_addr = self.net.local_addr();
        peers.sort_by_key(|p| p.addr.unwrap_or(local_addr));
        Box::new(Connection {
            peers,
            app: None,
            net: self.net,
            last_sync: None,
            last_ready: None,
            seed: None,
            started_at: None,
        })
    }

    pub fn update(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
        if !self.started {
            self.started = true;
            self.net.start()?;
        }
        if !self.stopped {
            let now = device.now();
            self.advertise(now)?;
        }
        for _ in 0..4 {
            let Some((addr, msg)) = self.net.recv()? else {
                break;
            };
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
        match req {
            Req::Hello => self.handle_hello(device, addr),
            Req::Intro => self.send_intro(device, addr),
            Req::Disconnect => self.handle_disconnect(addr),
            _ => Ok(()),
        }
    }

    fn handle_resp(&mut self, addr: Addr, resp: Resp) -> Result<(), NetcodeError> {
        match resp {
            Resp::Intro(intro) => self.handle_intro(addr, intro),
            _ => Ok(()),
        }
    }

    fn handle_hello(&mut self, device: &DeviceImpl, addr: Addr) -> Result<(), NetcodeError> {
        if !self.stopped && !self.peer_addrs.contains(&addr) {
            let res = self.peer_addrs.push(addr);
            if res.is_err() {
                return Err(NetcodeError::PeerListFull);
            }
        }
        self.send_intro(device, addr)?;
        Ok(())
    }

    fn handle_intro(&mut self, addr: Addr, intro: Intro) -> Result<(), NetcodeError> {
        if self.stopped {
            return Ok(());
        }
        for info in &self.peer_infos {
            if info.addr == addr {
                return Ok(());
            }
        }
        let mut intro = intro;
        if firefly_types::validate_id(&intro.name).is_err() {
            intro.name = "anonymous".try_into().unwrap();
        }
        let info = PeerInfo { addr, intro };
        let res = self.peer_infos.push(info);
        if res.is_err() {
            return Err(NetcodeError::PeerListFull);
        }
        Ok(())
    }

    fn handle_disconnect(&mut self, addr: Addr) -> Result<(), NetcodeError> {
        let mut name = heapless::String::try_from("???").unwrap();

        let maybe_index = self
            .peer_infos
            .iter()
            .enumerate()
            .find(|(_, info)| info.addr == addr);
        if let Some((peer_index, peer_info)) = maybe_index {
            name = peer_info.intro.name.clone();
            self.peer_infos.remove(peer_index);
        }

        let maybe_index = self
            .peer_addrs
            .iter()
            .enumerate()
            .find(|(_, peer_addr)| **peer_addr == addr);
        if let Some((index, _)) = maybe_index {
            self.peer_addrs.remove(index);
        }

        if self.stopped {
            return Err(NetcodeError::Disconnected(name));
        }
        Ok(())
    }

    fn send_intro(&mut self, _device: &DeviceImpl, addr: Addr) -> Result<(), NetcodeError> {
        let intro = self.me.clone();
        let msg = Message::Resp(Resp::Intro(intro));
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = msg.encode(&mut buf)?;
        self.net.send(addr, raw)?;
        Ok(())
    }

    fn send_disconnect(&mut self) -> Result<(), NetcodeError> {
        let msg = Message::Req(Req::Disconnect);
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = msg.encode(&mut buf)?;
        for addr in &self.peer_addrs {
            self.net.send(*addr, raw)?;
        }
        Ok(())
    }
}
