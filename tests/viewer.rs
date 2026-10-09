use std::time::{Duration, Instant};

use ac2_client::{
    ClientConfig,
    fake::{FakeDaemon, FakeOptions},
};
use ac2_proto::{Change, FrameData, Patch, Stream, Topic, samples, units::MeasId};
use ac2_remote::link::Link;

fn until(mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !check() {
        assert!(Instant::now() < deadline, "viewer worker timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn viewer_receives_grids_switches_subscription_and_marks_silence_stale() {
    let daemon = FakeDaemon::start(FakeOptions::default()).unwrap();
    let grid = samples::log_grid();
    let mut measurement = samples::state().measurements.remove(0);
    measurement.id = MeasId(1);
    measurement.running = true;
    measurement.grid_id = Some(grid.id());
    {
        let mut state = daemon.lock();
        state.grids.insert(grid.id(), grid.clone());
        state.commit(Change::Measurement(Patch::Set(measurement.clone())));
        measurement.id = MeasId(2);
        state.commit(Change::Measurement(Patch::Set(measurement)));
    }
    let link = Link::start(ClientConfig::new(daemon.endpoints(), "remote-test"));
    until(|| link.snapshot().transfers().len() == 2);
    link.select(Some(MeasId(1)));
    let topic = |id| Topic::Data {
        meas: MeasId(id),
        stream: Stream::Tf,
    };
    let mut seq = 0;
    let mut publish = |id| {
        seq += 1;
        let mut frame = samples::tf_frame();
        frame.stamp = daemon.lock().stamp(seq, Some(grid.id()));
        if let FrameData::Tf(tf) = &mut frame.data {
            tf.meas = MeasId(id);
        }
        daemon.lock().publish(&frame);
    };
    until(|| {
        publish(1);
        let snapshot = link.snapshot();
        snapshot.latest.get(&topic(1)).is_some() && snapshot.grids.contains_key(&grid.id())
    });
    assert_eq!(daemon.executions("grid.get"), 1);
    link.select(Some(MeasId(2)));
    until(|| {
        publish(1);
        publish(2);
        let latest = link.snapshot().latest;
        latest.get(&topic(2)).is_some() && latest.get(&topic(1)).is_none()
    });
    daemon.lock().ka_paused = true;
    until(|| {
        link.snapshot()
            .latest
            .get(&topic(2))
            .is_some_and(|f| f.stale)
    });
    until(|| {
        link.snapshot()
            .mirror
            .is_some_and(|m| !m.responding(Instant::now()))
    });
    let requests = daemon.lock().requests.clone();
    for (op, _) in requests {
        assert!(
            matches!(op, "hello" | "state.snapshot" | "state.since" | "grid.get"),
            "viewer sent mutating command: {op}"
        );
    }
    let worker_snapshot = std::sync::Arc::downgrade(&link.snapshot);
    drop(link);
    until(|| worker_snapshot.upgrade().is_none());
}

#[test]
fn disconnect_cancels_pending_handshake() {
    let daemon = FakeDaemon::start(FakeOptions::default()).unwrap();
    daemon.lock().mute = true;
    let link = Link::start(ClientConfig::new(daemon.endpoints(), "cancel-test"));
    std::thread::sleep(Duration::from_millis(100));
    let worker_snapshot = std::sync::Arc::downgrade(&link.snapshot);
    let start = Instant::now();
    drop(link);
    until(|| worker_snapshot.upgrade().is_none());
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "disconnect waited for handshake timeout"
    );
}

#[test]
fn viewer_switches_between_spectrum_rta_spl_and_math_streams() {
    use ac2_proto::model::{MeasKind, SpecAveraging, SpectrumConfig};
    use ac2_proto::units::Seconds;
    let daemon = FakeDaemon::start(FakeOptions::default()).unwrap();
    let mut measurement = samples::state().measurements.remove(0);
    measurement.id = MeasId(10);
    let mut frames: Vec<_> = samples::frames()
        .into_iter()
        .filter(|f| {
            matches!(
                f.data,
                FrameData::Spec(_) | FrameData::Rta(_) | FrameData::Spl(_)
            )
        })
        .collect();
    let rta_kind = samples::commands()
        .into_iter()
        .find_map(|c| match c {
            ac2_proto::Command::MeasUpdate { config, .. }
                if matches!(config.kind, MeasKind::Rta { .. }) =>
            {
                Some(config.kind)
            }
            _ => None,
        })
        .unwrap();
    let spec_kind = MeasKind::Spectrum {
        config: SpectrumConfig {
            input: 0,
            fft_len: 1024,
            window: ac2_proto::model::Window::Hann,
            averaging: SpecAveraging::Exponential {
                time_constant: Seconds(1.0),
            },
            smoothing: None,
        },
    };
    let link = Link::start(ClientConfig::new(daemon.endpoints(), "all-views"));
    for (seq, frame) in frames.iter_mut().enumerate() {
        let kind = match &mut frame.data {
            FrameData::Spec(f) => {
                f.meas = measurement.id;
                spec_kind.clone()
            }
            FrameData::Rta(f) => {
                f.meas = measurement.id;
                rta_kind.clone()
            }
            FrameData::Spl(f) => {
                f.meas = measurement.id;
                samples::spl_measurement().config.kind
            }
            _ => unreachable!(),
        };
        measurement.config.kind = kind;
        let stream = measurement.config.kind.stream().unwrap();
        {
            let mut state = daemon.lock();
            for grid in samples::grids() {
                state.grids.insert(grid.id(), grid);
            }
            state.commit(Change::Measurement(Patch::Set(measurement.clone())));
            frame.stamp = state.stamp(seq as u64 + 1, frame.stamp.grid_id);
        }
        link.select(Some(measurement.id));
        let topic = Topic::Data {
            meas: measurement.id,
            stream,
        };
        until(|| {
            daemon.lock().publish(frame);
            link.snapshot().latest.get(&topic).is_some()
        });
    }
    let mut math = samples::math_measurement();
    math.id = MeasId(11);
    daemon
        .lock()
        .commit(Change::Measurement(Patch::Set(math.clone())));
    link.select(Some(math.id));
    let mut frame = samples::tf_frame();
    if let FrameData::Tf(f) = &mut frame.data {
        f.meas = math.id;
    }
    frame.stamp = daemon.lock().stamp(100, Some(samples::log_grid().id()));
    until(|| {
        daemon.lock().publish(&frame);
        link.snapshot()
            .latest
            .get(&Topic::Data {
                meas: math.id,
                stream: Stream::Tf,
            })
            .is_some()
    });
    // The same spectrum page receives FFT and RTA simultaneously.
    measurement.config.kind = spec_kind;
    let mut bands = measurement.clone();
    bands.id = MeasId(12);
    bands.config.kind = rta_kind;
    {
        let mut state = daemon.lock();
        state.commit(Change::Measurement(Patch::Set(measurement.clone())));
        state.commit(Change::Measurement(Patch::Set(bands.clone())));
    }
    link.select(Some(measurement.id));
    let mut spec = samples::frames()
        .into_iter()
        .find(|f| matches!(f.data, FrameData::Spec(_)))
        .unwrap();
    let mut rta = samples::frames()
        .into_iter()
        .find(|f| matches!(f.data, FrameData::Rta(_)))
        .unwrap();
    if let FrameData::Spec(f) = &mut spec.data {
        f.meas = measurement.id;
    }
    if let FrameData::Rta(f) = &mut rta.data {
        f.meas = bands.id;
    }
    spec.stamp = daemon.lock().stamp(101, spec.stamp.grid_id);
    rta.stamp = daemon.lock().stamp(102, rta.stamp.grid_id);
    until(|| {
        daemon.lock().publish(&spec);
        daemon.lock().publish(&rta);
        let snapshot = link.snapshot();
        snapshot
            .latest
            .get(&Topic::Data {
                meas: measurement.id,
                stream: Stream::Spec,
            })
            .is_some()
            && snapshot
                .latest
                .get(&Topic::Data {
                    meas: bands.id,
                    stream: Stream::Rta,
                })
                .is_some()
    });
    for (op, _) in &daemon.lock().requests {
        assert!(
            matches!(*op, "hello" | "state.snapshot" | "state.since" | "grid.get"),
            "mutating command: {op}"
        );
    }
}

#[test]
fn stored_slots_follow_visibility_rename_and_deletion() {
    use ac2_client::Client;
    use ac2_proto::{Command, ReplyBody};
    let daemon = FakeDaemon::start(FakeOptions::default()).unwrap();
    let measurement = samples::state().measurements.remove(0);
    daemon
        .lock()
        .commit(Change::Measurement(Patch::Set(measurement.clone())));
    let rt = tokio::runtime::Runtime::new().unwrap();
    let setup = rt
        .block_on(Client::connect(ClientConfig::new(
            daemon.endpoints(),
            "slot-setup",
        )))
        .unwrap();
    let meta = match rt
        .block_on(setup.call(Command::TraceCapture {
            meas: measurement.id,
            name: "Saved slot".into(),
            slot: Some(1),
        }))
        .unwrap()
    {
        ReplyBody::Trace(meta) => meta,
        other => panic!("unexpected capture reply: {other:?}"),
    };
    let link = Link::start(ClientConfig::new(daemon.endpoints(), "slot-viewer"));
    link.select(Some(measurement.id));
    until(|| link.snapshot().traces.contains_key(&meta.id));
    assert_eq!(daemon.executions("trace.get"), 1);
    let mut edit = meta.edit;
    edit.visible = false;
    rt.block_on(setup.call(Command::TraceUpdate {
        trace: meta.id,
        edit: edit.clone(),
    }))
    .unwrap();
    until(|| !link.snapshot().traces.contains_key(&meta.id));
    edit.visible = true;
    edit.name = "Renamed slot".into();
    rt.block_on(setup.call(Command::TraceUpdate {
        trace: meta.id,
        edit,
    }))
    .unwrap();
    until(|| {
        link.snapshot()
            .traces
            .get(&meta.id)
            .is_some_and(|t| t.meta.edit.name == "Renamed slot")
    });
    assert_eq!(daemon.executions("trace.get"), 2);
    rt.block_on(setup.call(Command::TraceDelete { trace: meta.id }))
        .unwrap();
    until(|| !link.snapshot().traces.contains_key(&meta.id));
    assert_eq!(daemon.executions("trace.capture"), 1);
    assert_eq!(daemon.executions("trace.update"), 2);
    assert_eq!(daemon.executions("trace.delete"), 1);
}

#[test]
fn viewer_reads_latest_stored_sweep_without_running_one() {
    use ac2_client::Client;
    use ac2_proto::{Command, ReplyBody};
    let daemon = FakeDaemon::start(FakeOptions::default()).unwrap();
    let measurement = samples::sweep_measurement();
    daemon
        .lock()
        .commit(Change::Measurement(Patch::Set(measurement.clone())));
    let rt = tokio::runtime::Runtime::new().unwrap();
    let setup = rt
        .block_on(Client::connect(ClientConfig::new(
            daemon.endpoints(),
            "sweep-setup",
        )))
        .unwrap();
    let lease = match rt
        .block_on(setup.call(Command::GenAcquire { force: false }))
        .unwrap()
    {
        ReplyBody::Lease(lease) => lease.lease_token,
        other => panic!("unexpected lease reply: {other:?}"),
    };
    rt.block_on(setup.call(Command::GenSet {
        lease_token: lease,
        desired: ac2_proto::model::GeneratorDesired {
            settings: samples::state().generator.settings.unwrap(),
            armed: true,
            firing: false,
        },
    }))
    .unwrap();
    rt.block_on(setup.call(Command::SweepRun {
        lease_token: lease,
        meas: measurement.id,
        name: Some("First".into()),
    }))
    .unwrap();
    let link = Link::start(ClientConfig::new(daemon.endpoints(), "sweep-viewer"));
    link.select(Some(measurement.id));
    until(|| link.snapshot().sweep.is_some());
    let first = link.snapshot().sweep.unwrap();
    assert!(first.sweep.is_some());
    assert!(link.snapshot().grids.contains_key(&first.meta.grid_id));
    assert_eq!(daemon.executions("sweep.run"), 1);
    rt.block_on(setup.call(Command::GenSet {
        lease_token: lease,
        desired: ac2_proto::model::GeneratorDesired {
            settings: samples::state().generator.settings.unwrap(),
            armed: true,
            firing: false,
        },
    }))
    .unwrap();
    rt.block_on(setup.call(Command::SweepRun {
        lease_token: lease,
        meas: measurement.id,
        name: Some("Second".into()),
    }))
    .unwrap();
    until(|| {
        link.snapshot()
            .sweep
            .is_some_and(|data| data.meta.id != first.meta.id)
    });
    assert_eq!(daemon.executions("sweep.run"), 2);
    assert_eq!(daemon.executions("trace.get"), 2);
}
