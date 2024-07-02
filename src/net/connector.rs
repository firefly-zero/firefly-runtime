use super::*;
use firefly_device::*;

const ADVERTISE_EVERY: Duration = Duration::from_ms(100);
const MAX_PEERS: usize = 7;
type Addr = <NetworkImpl as Network>::Addr;

pub(crate) struct MyInfo {
    pub name:    heapless::String<16>,
    pub version: u16,
}

pub(crate) struct PeerInfo {
    addr:    Addr,
    name:    heapless::String<16>,
    version: u16,
}

pub(crate) struct Connector {
    me:                 MyInfo,
    net:                NetworkImpl,
    last_advertisement: Option<Instant>,
    peer_addrs:         heapless::Vec<Addr, MAX_PEERS>,
    peer_infos:         heapless::Vec<PeerInfo, MAX_PEERS>,
    started:            bool,
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
        }
    }

    pub fn update(&mut self, device: &DeviceImpl) {
        let res = self.update_inner(device);
        if let Err(err) = res {
            device.log_error("netcode", err);
        }
    }

    // pub fn finalize(self) {
    //     todo!()
    // }

    fn update_inner(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
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
        raw: heapless::Vec<u8, 64>,
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

    fn send_intro(&mut self, device: &DeviceImpl, addr: Addr) -> Result<(), NetcodeError> {
        let intro = Intro {
            name:    self.me.name.clone(),
            version: self.me.version,
        };
        let msg = Message::Resp(intro.into());
        let mut buf = [0u8, 64];
        let raw = match msg.encode(&mut buf) {
            Ok(raw) => raw,
            Err(err) => return Err(NetcodeError::Serialize(err)),
        };
        self.net.send(addr, raw)?;
        Ok(())
    }
}
