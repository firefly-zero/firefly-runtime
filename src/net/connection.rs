use super::*;
use crate::utils::{read_all, read_into, write_all};
use crate::FullID;
use alloc::boxed::Box;
use embedded_io::{Read, Write};
use firefly_hal::*;
use firefly_types::{Encode, Stats};
use ring::RingBuf;

const SYNC_EVERY: Duration = Duration::from_ms(100);
const READY_EVERY: Duration = Duration::from_ms(100);
const START_TIMEOUT: Duration = Duration::from_ms(10_000);
const MAX_PEERS: usize = 8;
const MSG_SIZE: usize = 64;

pub(crate) struct Peer {
    /// If address is None, the peer is the current device.
    pub addr: Option<Addr>,
    /// The human-readable name of the device.
    pub name: heapless::String<16>,
    /// Not None when the peer is ready to start the selected app.
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
    /// The peer's stash: a shared state preserved across games.
    pub stash: alloc::vec::Vec<u8>,
    /// The peer's true RNG seed.
    pub seed: u32,
}

pub(crate) enum ConnectionStatus {
    /// Waiting for one of the players to pick an app to launch.
    Waiting,
    /// Ready to launch an app, waiting for everyone to confirm the choice.
    Ready,
    /// Everyone agreed to play, launch the app.
    Launching,
    /// We wanted to start an app but didn't get intros from some peers.
    Timeout,
}

/// Connection is a result of connector.
///
/// If you play games with friends, you establish the connection once
/// at the beginning of the evening and it stays on as long as
/// all the devices are turned on.
///
/// This object is allocated while your are in the launcher.
/// Its job is to launch an app for everyone when someone picks one to play.
pub(crate) struct Connection<'a> {
    /// In the initial state, a prepared intro for the current device.
    /// Later, when the app to be launched is known, contains the id of the app to launch
    /// and the intro moves into [`Peer`] corresponding to the local device.
    pub app: Option<FullID>,
    pub seed: Option<u32>,
    pub peers: heapless::Vec<Peer, MAX_PEERS>,
    pub(super) net: NetworkImpl<'a>,
    /// The last time when the device checked if other devices are ready to start.
    pub(super) last_sync: Option<Instant>,
    /// The last time when the device announced that it's ready to start the app.
    pub(super) last_ready: Option<Instant>,
    /// The moment when we knew which app to launch.
    pub(super) started_at: Option<Instant>,
}

impl<'a> Connection<'a> {
    pub fn update(&mut self, device: &mut DeviceImpl) -> ConnectionStatus {
        if let Some(started_at) = self.started_at {
            let now = device.now();
            if now - started_at > START_TIMEOUT {
                self.started_at = None; // don't do timeout check if update called again
                return ConnectionStatus::Timeout;
            }
        }
        let res = self.update_inner(device);
        if let Err(err) = res {
            device.log_error("netcode", &err);
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

    /// Disconnect from multiplayer.
    ///
    /// Can be called from menu in launcher if connected to multiplayer.
    pub fn disconnect(mut self) -> Result<(), NetcodeError> {
        self.broadcast(Req::Disconnect.into())?;
        self.net.stop()?;
        Ok(())
    }

    pub fn set_app(&mut self, device: &mut DeviceImpl, app: FullID) -> Result<(), NetcodeError> {
        if self.app.is_some() {
            // App already was picked, cannot pick a new one.
            //
            // TODO: Broadcast an error if the new app doesn't match the old one.
            return Ok(());
        };
        let seed = self.get_seed(device);
        let intro = make_intro(device, &app, seed)?;
        // TODO: reduce the amount of `.clone()` in this function.
        let resp = Resp::Start(Start {
            id: app.clone(),
            badges: intro.badges.clone(),
            scores: intro.scores.clone(),
            stash: intro.stash.clone().into(),
            seed: intro.seed,
        });
        self.broadcast(resp.into())?;
        self.app = Some(app);
        self.started_at = Some(device.now());
        let me = self.get_me_mut();
        me.intro = Some(intro);
        Ok(())
    }

    /// The initial seed that must be used when starting the app.
    ///
    /// When called for the first time, will fetch a random value from
    /// the device's true RNG. After that, will cache the value
    /// to ensure it's the same in all intros.
    fn get_seed(&mut self, device: &mut DeviceImpl) -> u32 {
        match self.seed {
            Some(seed) => seed,
            None => {
                let seed = device.random();
                self.seed = Some(seed);
                seed
            }
        }
    }

    pub(crate) fn finalize(self, device: &mut DeviceImpl) -> Box<FrameSyncer<'a>> {
        let mut peers = heapless::Vec::<FSPeer, 8>::new();
        let mut seed = 0;
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
                // TODO: don't keep these in memory for the current device.
                // They are already stored in State.
                badges: intro.badges,
                scores: intro.scores,
                stash: intro.stash,
            };
            peers.push(peer).ok().unwrap();
            seed ^= intro.seed;
        }
        Box::new(FrameSyncer {
            peers,
            net: self.net,
            last_sync: None,
            frame: 0,
            last_advance: None,
            device_seed: self.seed.unwrap(),
            shared_seed: seed,
            app: self.app.unwrap(),
        })
    }

    fn update_inner(&mut self, device: &mut DeviceImpl) -> Result<(), NetcodeError> {
        let now = device.now();
        self.sync(now)?;
        self.send_ready(now)?;
        for _ in 0..4 {
            let Some((addr, msg)) = self.net.recv()? else {
                break;
            };
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
    fn send_ready(&mut self, now: Instant) -> Result<(), NetcodeError> {
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
            stash: intro.stash.clone().into_boxed_slice(),
            seed: intro.seed,
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
        device: &mut DeviceImpl,
        addr: Addr,
        raw: Box<[u8]>,
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
        match req {
            Req::Start => self.handle_start_req(addr)?,
            Req::Disconnect => self.handle_disconnect(addr)?,
            _ => {}
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
            stash: intro.stash.clone().into_boxed_slice(),
            seed: intro.seed,
        };
        let resp = Message::Resp(Resp::Start(resp));
        let mut buf = alloc::vec![0u8; MSG_SIZE];
        let raw = resp.encode(&mut buf)?;
        self.net.send(addr, raw)?;
        Ok(())
    }

    fn handle_disconnect(&mut self, addr: Addr) -> Result<(), NetcodeError> {
        let mut name = heapless::String::try_from("???").unwrap();

        let maybe_index = self
            .peers
            .iter()
            .enumerate()
            .find(|(_, peer)| peer.addr == Some(addr));
        if let Some((index, _)) = maybe_index {
            let peer = self.peers.remove(index);
            name = peer.name;
        }

        Err(NetcodeError::Disconnected(name))
    }

    fn handle_resp(
        &mut self,
        device: &mut DeviceImpl,
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
        device: &mut DeviceImpl,
        intro: Start,
        addr: Addr,
    ) -> Result<(), NetcodeError> {
        self.set_app(device, intro.id)?;
        if let Some(peer) = self.get_peer(addr) {
            peer.intro = Some(AppIntro {
                badges: intro.badges,
                scores: intro.scores,
                stash: intro.stash.to_vec(),
                seed: intro.seed,
            });
        };
        Ok(())
    }

    /// Send the message to all connected peers.
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
fn get_friend_id(device: &mut DeviceImpl, device_name: &str) -> Option<u16> {
    if device_name.len() > 16 {
        return None;
    }
    let device_name = device_name.as_bytes();
    let mut dir = device.open_dir(&["sys"]).ok()?;
    let Ok(mut stream) = dir.open_file("friends") else {
        let mut stream = dir.create_file("friends").ok()?;
        stream.write(&[device_name.len() as u8]).ok()?;
        write_all(stream, device_name).ok()?;
        return Some(1);
    };

    // check if the device name is already in the list of friends
    let mut buf = [0u8; 17];
    let mut i = 1;
    loop {
        match stream.read(&mut buf[..1]) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let size = usize::from(buf[0]);
        if size == 0 {
            continue;
        }
        read_into(&mut stream, &mut buf[1..=size]).ok()?;
        if &buf[1..=size] == device_name {
            return Some(i);
        }
        i += 1;
    }

    let mut stream = dir.append_file("friends").ok()?;
    stream.write(&[device_name.len() as u8]).ok()?;
    write_all(stream, device_name).ok()?;
    Some(i + 1)
}

pub(crate) fn make_intro(
    device: &mut DeviceImpl,
    id: &FullID,
    seed: u32,
) -> Result<AppIntro, NetcodeError> {
    let dir_path = &["data", id.author(), id.app()];
    let mut dir = match device.open_dir(dir_path) {
        Ok(dir) => dir,
        Err(err) => return Err(NetcodeError::StashFileError(err)),
    };

    // Read stash file.
    let stash = match dir.open_file("stash") {
        Ok(stream) => match read_all(stream) {
            Ok(stream) => stream,
            Err(err) => return Err(NetcodeError::StashFileError(err.into())),
        },
        Err(FSError::NotFound) => alloc::vec::Vec::new(),
        Err(err) => return Err(NetcodeError::StashFileError(err)),
    };

    // Read stats file.
    let stream = match dir.open_file("stats") {
        Ok(stream) => stream,
        Err(FSError::NotFound) => {
            return Ok(AppIntro {
                badges: Box::new([]),
                scores: Box::new([]),
                stash,
                seed,
            });
        }
        Err(err) => return Err(NetcodeError::StatsFileError(err)),
    };
    let Ok(raw) = read_all(stream) else {
        return Err(NetcodeError::StatsError("cannot read stats file"));
    };
    if raw.is_empty() {
        return Err(NetcodeError::StatsError("file is empty"));
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
        stash,
        seed,
    };
    Ok(intro)
}
