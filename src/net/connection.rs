use crate::FullID;

use super::*;
use firefly_device::*;

const SYNC_EVERY: Duration = Duration::from_ms(100);
const READY_EVERY: Duration = Duration::from_ms(100);
const MAX_PEERS: usize = 8;
const MSG_SIZE: usize = 64;
type Addr = <NetworkImpl as Network>::Addr;

pub(crate) struct Peer {
    /// If address is None, the peer is the current device.
    pub addr: Option<Addr>,
    pub name: heapless::String<16>,
    pub ready: bool,
}

pub(crate) enum ConnectionStatus {
    /// Waiting for one of the players to pick an app to launch.
    Waiting,
    /// Ready to launch an app, waiting for everyone to confirm the choice.
    Ready,
    /// Everyone agreed to play, launch the app.
    Launching,
}

/// Connection is a result of connector.
///
/// If you play games with friends, you establish the connection once
/// at the beginning of the evening and it stays on as long as
/// all the devices are turned on.
///
/// This object is allocated while your are in the launcher.
/// Its job is to launch an app for everyone when someone picks one to play.
pub(crate) struct Connection {
    pub app: Option<FullID>,
    pub peers: heapless::Vec<Peer, MAX_PEERS>,
    pub(super) net: NetworkImpl,
    pub(super) last_sync: Option<Instant>,
    pub(super) last_ready: Option<Instant>,
}

impl Connection {
    pub fn update(&mut self, device: &DeviceImpl) -> ConnectionStatus {
        let res = self.update_inner(device);
        if let Err(err) = res {
            device.log_error("netcode", err);
        }
        let all_ready = self.peers.iter().all(|p| p.ready);
        if all_ready {
            return ConnectionStatus::Launching;
        }
        match self.app {
            Some(_) => ConnectionStatus::Ready,
            None => ConnectionStatus::Waiting,
        }
    }

    pub fn set_app(&mut self, app: FullID) -> Result<(), NetcodeError> {
        if self.app.is_some() {
            return Ok(());
        }
        self.broadcast(Resp::Start(app.clone()).into())?;
        self.app = Some(app);
        for peer in self.peers.iter_mut() {
            if peer.addr.is_none() {
                peer.ready = true;
            }
        }
        Ok(())
    }

    fn update_inner(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
        let now = device.now();
        self.sync(now)?;
        self.ready(now)?;
        if let Some((addr, msg)) = self.net.recv()? {
            self.handle_message(addr, msg)?;
        }
        Ok(())
    }

    /// Ask other devices if they already started.
    fn sync(&mut self, now: Instant) -> Result<(), NetcodeError> {
        if let Some(prev) = self.last_sync {
            if now - prev < SYNC_EVERY {
                return Ok(());
            }
        }
        self.last_sync = Some(now);
        self.broadcast(Req::Start.into())?;
        Ok(())
    }

    /// Tell other devices if we are ready to start.
    fn ready(&mut self, now: Instant) -> Result<(), NetcodeError> {
        // Say we're ready only if we are actually ready:
        // if we know the next app to launch.
        let Some(app) = &self.app else {
            return Ok(());
        };
        if let Some(prev) = self.last_ready {
            if now - prev < READY_EVERY {
                return Ok(());
            }
        }
        self.last_ready = Some(now);
        self.broadcast(Resp::Start(app.clone()).into())?;
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
        if matches!(req, Req::Start) {
            if let Some(app) = &self.app {
                let msg = Message::Resp(Resp::Start(app.clone()));
                let mut buf = alloc::vec![0u8; MSG_SIZE];
                let raw = msg.encode(&mut buf)?;
                self.net.send(addr, raw)?;
            }
        }
        Ok(())
    }

    fn handle_resp(&mut self, addr: Addr, resp: Resp) -> Result<(), NetcodeError> {
        match resp {
            Resp::Start(app) => {
                self.set_app(app)?;
                if let Some(peer) = self.get_peer(addr) {
                    peer.ready = true;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn broadcast(&mut self, msg: Message) -> Result<(), NetcodeError> {
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = msg.encode(&mut buf)?;
        for peer in &self.peers {
            if let Some(addr) = peer.addr {
                self.net.send(addr, raw)?;
            }
        }
        Ok(())
    }

    fn get_peer(&mut self, addr: Addr) -> Option<&mut Peer> {
        for peer in &mut self.peers {
            if let Some(peer_addr) = peer.addr {
                if peer_addr == addr {
                    return Some(peer);
                }
            }
        }
        None
    }
}
