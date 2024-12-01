use super::*;
use crate::FullID;
use alloc::boxed::Box;
use embedded_io::{Read, Write};
use firefly_hal::*;
use firefly_types::{Encode, Stats};
use ring::RingBuf;

const SYNC_EVERY: Duration = Duration::from_ms(100);
const READY_EVERY: Duration = Duration::from_ms(100);
const MAX_PEERS: usize = 8;
const MSG_SIZE: usize = 64;
type Addr = <NetworkImpl as Network>::Addr;

pub(crate) struct Peer {
    /// If address is None, the peer is the current device.
    pub addr: Option<Addr>,
    /// The human-readable name of the device.
    pub name: heapless::String<16>,
    /// Not None when the peer is ready to start teh selected app.
    pub intro: Option<AppIntro>,
}

impl Peer {
    fn ready(&self) -> bool {
        self.intro.is_some()
    }
}

#[derive(Clone)]
pub(crate) struct AppIntro {
    /// The peer's progress for each badge.
    badges: Box<[u16]>,
    /// The peer's top score for each board.
    scores: Box<[i16]>,
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
    /// In the initial state, a prepared intro for the current device.
    /// Later, when the app to be launched is known, contains the id of the app to launch
    /// and the intro moves into [`Peer`] corresponding to the local device.
    pub app: Option<FullID>,
    pub peers: heapless::Vec<Peer, MAX_PEERS>,
    pub(super) net: NetworkImpl,
    /// The last time when the device checked if other devices are ready to start.
    pub(super) last_sync: Option<Instant>,
    /// The last time when the device announced that it's ready to start the app.
    pub(super) last_ready: Option<Instant>,
}

impl Connection {
    pub fn update(&mut self, device: &DeviceImpl) -> ConnectionStatus {
        let res = self.update_inner(device);
        if let Err(err) = res {
            device.log_error("netcode", err);
        }
        let all_ready = self.peers.iter().all(|p| p.ready());
        if all_ready {
            return ConnectionStatus::Launching;
        }
        match self.app {
            Some(_) => ConnectionStatus::Ready,
            None => ConnectionStatus::Waiting,
        }
    }

    pub fn set_app(&mut self, device: &DeviceImpl, app: FullID) -> Result<(), NetcodeError> {
        if self.app.is_some() {
            // App already was picked, cannot pick a new one.
            //
            // TODO: Broadcast an error if the new app doesn't match the old one.
            return Ok(());
        };
        let intro = self.make_intro(device, &app)?;
        // TODO: reduce the amount of `.clone()` in this function.
        let resp = Resp::Start(Start {
            id: app.clone(),
            badges: intro.badges.clone(),
            scores: intro.scores.clone(),
        });
        self.broadcast(resp.into())?;
        self.app = Some(app);
        for peer in self.peers.iter_mut() {
            if peer.addr.is_none() {
                peer.intro = Some(intro.clone());
            }
        }
        Ok(())
    }

    fn make_intro(&mut self, device: &DeviceImpl, id: &FullID) -> Result<AppIntro, NetcodeError> {
        let me = self.get_me_mut();
        debug_assert!(me.intro.is_none());

        // check if the stats file even exists
        let stats_path = &["data", id.author(), id.app(), "stats"];
        let Some(size) = device.get_file_size(stats_path) else {
            return Ok(AppIntro {
                badges: Box::new([]),
                scores: Box::new([]),
            });
        };

        if size == 0 {
            return Err(NetcodeError::StatsError("file is empty"));
        }
        let Some(mut stream) = device.open_file(stats_path) else {
            return Err(NetcodeError::StatsError("cannot open stats file"));
        };
        let mut raw = alloc::vec![0u8; size as usize];
        let res = stream.read(&mut raw);
        if res.is_err() {
            return Err(NetcodeError::StatsError("cannot read stats file"));
        }
        let stats = match Stats::decode(&raw) {
            Ok(stats) => stats,
            Err(_) => {
                return Err(NetcodeError::StatsError("cannot decode stats"));
            }
        };

        let mut badges = alloc::vec::Vec::new();
        let mut scores = alloc::vec::Vec::new();
        for badge in stats.badges {
            badges.push(badge.done);
        }
        for score in stats.scores {
            scores.push(score.me[0]);
        }

        let intro = AppIntro {
            badges: badges.into_boxed_slice(),
            scores: scores.into_boxed_slice(),
        };
        Ok(intro)
    }

    pub(crate) fn finalize(self, device: &DeviceImpl) -> FrameSyncer {
        let mut peers = heapless::Vec::<FSPeer, 8>::new();
        for peer in self.peers {
            let intro = peer.intro.unwrap();
            let friend_id = if peer.addr.is_none() {
                None
            } else {
                // TODO: don't open the file again for each peer. Detect all IDs in one go.
                get_friend_id(device, peer.name.as_str())
            };
            let peer = FSPeer {
                addr: peer.addr,
                name: peer.name,
                states: RingBuf::new(),
                friend_id,
                badges: intro.badges,
                scores: intro.scores,
            };
            peers.push(peer).ok().unwrap();
        }
        FrameSyncer {
            peers,
            net: self.net,
            last_sync: None,
            frame: 0,
            last_advance: None,
        }
    }

    fn update_inner(&mut self, device: &DeviceImpl) -> Result<(), NetcodeError> {
        let now = device.now();
        self.sync(now)?;
        self.ready(now)?;
        if let Some((addr, msg)) = self.net.recv()? {
            self.handle_message(device, addr, msg)?;
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
        let me = self.get_me();
        let intro = me.intro.as_ref().unwrap();
        let resp = Resp::Start(Start {
            id: app.clone(),
            badges: intro.badges.clone(),
            scores: intro.scores.clone(),
        });
        self.broadcast(resp.into())?;
        Ok(())
    }

    fn get_me(&self) -> &Peer {
        for peer in &self.peers {
            if peer.addr.is_none() {
                return peer;
            }
        }
        unreachable!("could not find the current device in the list of peers");
    }

    fn get_me_mut(&mut self) -> &mut Peer {
        for peer in &mut self.peers {
            if peer.addr.is_none() {
                return peer;
            }
        }
        unreachable!("could not find the current device in the list of peers");
    }

    fn handle_message(
        &mut self,
        device: &DeviceImpl,
        addr: Addr,
        raw: heapless::Vec<u8, MSG_SIZE>,
    ) -> Result<(), NetcodeError> {
        if !self.peers.iter().any(|p| p.addr == Some(addr)) {
            return Err(NetcodeError::UnknownPeer);
        }
        let msg = Message::decode(&raw)?;
        match msg {
            Message::Req(req) => self.handle_req(addr, req),
            Message::Resp(resp) => self.handle_resp(device, addr, resp),
        }
    }

    fn handle_req(&mut self, addr: Addr, req: Req) -> Result<(), NetcodeError> {
        if matches!(req, Req::Start) {
            self.handle_start_req(addr)?;
        }
        Ok(())
    }

    /// Handle a start request.
    ///
    /// The request is sent by other devices to check if the current device
    /// is ready to start an app.
    fn handle_start_req(&mut self, addr: Addr) -> Result<(), NetcodeError> {
        let Some(app) = &self.app else {
            return Ok(());
        };
        let me = self.get_me();
        let Some(intro) = &me.intro else {
            return Ok(());
        };
        let resp = Start {
            id: app.clone(),
            badges: intro.badges.clone(),
            scores: intro.scores.clone(),
        };
        let resp = Message::Resp(resp.into());
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = resp.encode(&mut buf)?;
        self.net.send(addr, raw)?;
        Ok(())
    }

    fn handle_resp(
        &mut self,
        device: &DeviceImpl,
        addr: Addr,
        resp: Resp,
    ) -> Result<(), NetcodeError> {
        if let Resp::Start(intro) = resp {
            self.handle_start_resp(device, intro, addr)?;
        }
        Ok(())
    }

    /// Handle a start response.
    ///
    /// The response arrives when another device is ready to start an app.
    /// This response contains info about the app that needs to be started
    /// as well as some app-specific peer info, like the progress earning badges.
    fn handle_start_resp(
        &mut self,
        device: &DeviceImpl,
        intro: Start,
        addr: Addr,
    ) -> Result<(), NetcodeError> {
        self.set_app(device, intro.id)?;
        if let Some(peer) = self.get_peer(addr) {
            peer.intro = Some(AppIntro {
                badges: intro.badges,
                scores: intro.scores,
            });
        };
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

/// Get the ID that can be used to referer to the device from scores (`FriendScore`).
fn get_friend_id(device: &DeviceImpl, device_name: &str) -> Option<u16> {
    if device_name.len() > 16 {
        return None;
    }
    let device_name = device_name.as_bytes();
    let path = &["sys", "friends"];
    let Some(mut stream) = device.open_file(path) else {
        let mut stream = device.create_file(path)?;
        stream.write(&[device_name.len() as u8]).ok()?;
        stream.write(device_name).ok()?;
        return Some(1);
    };

    // check if the device name is already in the list of friends
    let mut buf = [0u8; 17];
    let mut i = 1;
    loop {
        let res = stream.read(&mut buf[..1]);
        if res.is_err() {
            break;
        }
        let size = usize::from(buf[0]);
        stream.read(&mut buf[1..=size]).ok()?;
        if &buf[1..=size] == device_name {
            return Some(i);
        }
        i += 1;
    }

    let mut stream = device.append_file(path)?;
    stream.write(&[device_name.len() as u8]).ok()?;
    stream.write(device_name).ok()?;
    Some(i + 1)
}
