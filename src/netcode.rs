use crate::message::*;
use firefly_device::*;

const ADVERTISE_EVERY: Duration = Duration::from_ms(100);
type Addr = <NetworkImpl as Network>::Addr;

pub(crate) struct Connector {
    net:                NetworkImpl,
    last_advertisement: Option<Instant>,
    peers:              heapless::Vec<Addr, 4>,
}

impl Connector {
    pub fn new(net: NetworkImpl) -> Self {
        Self {
            net,
            last_advertisement: None,
            peers: heapless::Vec::new(),
        }
    }

    pub fn update(&mut self, device: &DeviceImpl) {
        let res = self.update_inner(device);
        if let Err(err) = res {
            device.log_error("netcode", err);
        }
    }

    pub fn finalize(self) {
        todo!()
    }

    fn update_inner(&mut self, device: &DeviceImpl) -> Result<(), NetworkError> {
        let now = device.now();
        self.advertise(now)?;
        if let Some((addr, msg)) = self.net.recv()? {
            self.handle_message(addr, msg)?;
        }
        Ok(())
    }

    fn advertise(&mut self, now: Instant) -> Result<(), NetworkError> {
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
        addr: Addr,
        raw: heapless::Vec<u8, 64>,
    ) -> Result<(), NetworkError> {
        if !self.peers.contains(&addr) {
            let res = self.peers.push(addr);
            if res.is_err() {
                todo!();
            }
            self.greet_peer(addr)?;
        }
        if raw == b"HELLO" {
            return Ok(());
        }
        // let msg = Message::deserialize(raw)?;
        todo!()
    }

    fn greet_peer(&mut self, addr: Addr) -> Result<(), NetworkError> {
        todo!()
    }
}
