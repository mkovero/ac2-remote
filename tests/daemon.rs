//! Opt-in end-to-end CURVE test. Only starts ac2d's fake audio backend.
use ac2_client::{Client, ClientConfig, Endpoints, KeyDir, RemoteAddr};
use ac2_proto::{Command, ReplyBody, Stream, Topic, model::*, samples};
use ac2_remote::link::Link;
use std::{
    net::TcpListener,
    process::{Child, Command as ProcessCommand},
    time::{Duration, Instant},
};

struct Daemon(Child);
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires AC2D_BIN pointing to a protocol-matching ac2d binary"]
async fn encrypted_daemon_transfer() -> Result<(), Box<dyn std::error::Error>> {
    let binary = std::env::var("AC2D_BIN")?;
    let directory = tempfile::tempdir()?;
    let keys = KeyDir::new(directory.path().join("client"));
    let (client_keys, _) = keys.ensure_client_keypair()?;
    let setup_keys = ac2_zmq::KeyPair::generate()?;
    let server_keys = ac2_zmq::KeyPair::generate()?;
    let key_file = directory.path().join("server.key");
    std::fs::write(
        &key_file,
        format!(
            "public {}\nsecret {}\n",
            server_keys.public.to_z85(),
            server_keys.secret.to_z85()
        ),
    )?;
    let authorized = directory.path().join("authorized_clients");
    std::fs::write(
        &authorized,
        format!("host-setup {}\n", setup_keys.public.to_z85()),
    )?;
    let (control, data) = loop {
        let control = TcpListener::bind("127.0.0.1:0")?;
        let port = control.local_addr()?.port();
        if let Some(next) = port.checked_add(1)
            && let Ok(data) = TcpListener::bind(("127.0.0.1", next))
        {
            break (control, data);
        }
    };
    let port = control.local_addr()?.port();
    drop((control, data));
    let log = std::fs::File::create(directory.path().join("daemon.log"))?;
    let mut daemon = Daemon(
        ProcessCommand::new(binary)
            .args([
                "--backend",
                "fake",
                "--listen",
                &format!("tcp://127.0.0.1:{port}"),
                "--no-mdns",
                "--no-autosave",
            ])
            .arg("--key-file")
            .arg(key_file)
            .arg("--authorized")
            .arg(authorized)
            .env("AC2_CONFIG_DIR", directory.path().join("config"))
            .env("AC2_DATA_DIR", directory.path().join("data"))
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()?,
    );
    let addr: RemoteAddr = format!("127.0.0.1:{port}").parse()?;
    keys.pin_server(&addr.host, server_keys.public)?;
    let mut config = ClientConfig::new(Endpoints::remote(&addr), "remote-smoke");
    config.curve = Some(keys.curve_client(&addr.host)?);
    let mut setup_config = config.clone();
    setup_config.curve = Some(ac2_zmq::CurveClient {
        keys: setup_keys,
        server_key: server_keys.public,
    });
    let client = Client::connect(setup_config).await?;
    client.wait_synced(Duration::from_secs(8)).await?;
    client
        .call(Command::SessionOpen {
            config: SessionConfig {
                backend: Some(BackendKind::Fake),
                input_device: DeviceSelector::Default,
                output_device: DeviceSelector::Default,
                input_channels: vec![0, 1],
                output_channels: 2,
                sample_rate_hz: Some(48_000),
                buffer_frames: Some(256),
                loopback: None,
            },
        })
        .await?;
    let measurement = samples::state().measurements.remove(0);
    let ReplyBody::Measurement(measurement) = client
        .call(Command::MeasCreate {
            config: measurement.config,
        })
        .await?
    else {
        return Err("expected measurement".into());
    };
    client
        .call(Command::MeasStart {
            meas: measurement.id,
        })
        .await?;
    let link = Link::start(config);
    link.select(Some(measurement.id));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let ReplyBody::Server(info) = client.call(Command::ServerInfo).await? else {
            return Err("expected server info".into());
        };
        let seen = match info.mode {
            ServerMode::Network { refused, .. } => refused
                .iter()
                .any(|r| r.key.as_deref() == Some(client_keys.public.to_z85().as_str())),
            _ => false,
        };
        // Let the first handshake time out before granting: retries must recover without
        // requiring the phone operator to tap Connect again.
        if seen && link.snapshot().error.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            return Err("phone key never appeared under Refused keys".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    client
        .call(Command::ServerAuthorize {
            name: "phone".into(),
            key: client_keys.public.to_z85(),
        })
        .await?;
    let topic = Topic::Data {
        meas: measurement.id,
        stream: Stream::Tf,
    };
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(exit) = daemon.0.try_wait()? {
            return Err(format!(
                "daemon exited: {exit}; {}",
                std::fs::read_to_string(directory.path().join("daemon.log"))?
            )
            .into());
        }
        let snapshot = link.snapshot();
        if let Some(frame) = snapshot.latest.get(&topic)
            && frame
                .frame
                .stamp
                .grid_id
                .is_some_and(|id| snapshot.grids.contains_key(&id))
        {
            assert!(
                snapshot
                    .mirror
                    .is_some_and(|m| m.synced() && m.responding(Instant::now()))
            );
            eprintln!(
                "Refused phone authorized without key entry or daemon restart; CURVE frame and grid received"
            );
            break;
        }
        if Instant::now() >= deadline {
            return Err("no transfer frame from encrypted daemon".into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Ok(())
}
