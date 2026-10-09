use ac2_discovery::{Browser, Options, Rig, RigTable};
use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

#[derive(Default, Clone)]
pub struct Snapshot {
    pub rigs: Vec<Rig>,
    pub error: Option<String>,
    pub invalid: usize,
}
pub struct Discovery {
    snapshot: Arc<Mutex<Snapshot>>,
    stop: Option<mpsc::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Discovery {
    pub fn start() -> Self {
        Self::with_options(Options::default())
    }
    fn with_options(options: Options) -> Self {
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let output = snapshot.clone();
        let (stop, stopped) = mpsc::channel::<()>();
        let worker = std::thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                #[cfg(target_os = "android")]
                let _multicast = crate::multicast::MulticastLock::acquire()?;
                let browser = Browser::start(&options).map_err(|e| e.to_string())?;
                let mut table = RigTable::default();
                loop {
                    for update in browser.poll() {
                        table.apply(update);
                    }
                    *output.lock().unwrap() = Snapshot {
                        rigs: table.rigs().into_iter().cloned().collect(),
                        error: None,
                        invalid: table.invalid().count(),
                    };
                    match stopped.recv_timeout(Duration::from_millis(200)) {
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        _ => break,
                    }
                }
                Ok(())
            })();
            if let Err(error) = result {
                output.lock().unwrap().error = Some(error);
            }
        });
        Self {
            snapshot,
            stop: Some(stop),
            worker: Some(worker),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().unwrap().clone()
    }
}
impl Drop for Discovery {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        // Release the multicast lock before android-activity tears down its global context.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac2_discovery::{Advert, Advertiser, Bind};
    use std::{
        net::{Ipv4Addr, UdpSocket},
        time::Instant,
    };

    #[test]
    fn browse_receives_a_pairable_public_key_and_releases_worker() {
        let port = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let options = Options {
            mdns_port: port,
            loopback_only: true,
        };
        let discovery = Discovery::with_options(options.clone());
        let key = ac2_zmq::PublicKey::from_bytes([7; 32]);
        let advert = Advert {
            name: "Viewer test".into(),
            version: "test".into(),
            proto: ac2_proto::PROTO_VERSION,
            fingerprint: key.fingerprint(),
            server_key: key.to_z85(),
        };
        let advertiser = Advertiser::start(
            &advert,
            47820,
            &Bind::Addr(Ipv4Addr::LOCALHOST.into()),
            &options,
            |_| {},
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let found = discovery.snapshot();
            assert!(found.error.is_none(), "{:?}", found.error);
            if let Some(rig) = found.rigs.first() {
                let connection = crate::pairing::Connection::from_rig(rig).unwrap();
                assert_eq!(connection.host, "127.0.0.1:47820");
                assert_eq!(connection.server_key, key.to_z85());
                break;
            }
            assert!(Instant::now() < deadline, "mDNS did not resolve the rig");
            std::thread::sleep(Duration::from_millis(20));
        }
        let worker = Arc::downgrade(&discovery.snapshot);
        drop(discovery);
        while worker.upgrade().is_some() {
            assert!(Instant::now() < deadline, "discovery worker did not stop");
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(advertiser);
    }
}
