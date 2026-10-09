use crate::link::Snapshot;
use ac2_proto::{
    FrameData, Stream, Topic,
    model::{MeasKind, Measurement},
};
use ac2_scene::{
    Theme, ViewState, Viewport,
    banner::Status,
    spectrograph::{SpectrographFrame, SpectrographHistory},
    time::Freshness,
};

pub fn cycle(view: &mut ViewState, kind: &MeasKind) {
    if kind.publishes_levels() {
        view.spectrum.mode = view.spectrum.mode.next();
    } else {
        match kind {
            MeasKind::Sweep { .. } => view.distortion.mode = view.distortion.mode.next(),
            MeasKind::Spl { config } => view.spl.mode = view.spl.mode.next(config.bands.is_some()),
            _ => {}
        }
    }
}

pub fn name(view: &ViewState, kind: &MeasKind) -> &'static str {
    use ac2_scene::view::{SpectrumMode, SplMode, SweepMode};
    if kind.publishes_levels() {
        match view.spectrum.mode {
            SpectrumMode::Spectrum => "Spectrum / RTA",
            SpectrumMode::Split => "Spectrum + spectrograph",
            SpectrumMode::Spectrograph => "Spectrograph",
        }
    } else {
        match kind {
            MeasKind::Sweep { .. } => match view.distortion.mode {
                SweepMode::Response => "Response / distortion",
                SweepMode::Ir => "Impulse response",
                SweepMode::Room => "Room parameters",
            },
            MeasKind::Spl { .. } => match view.spl.mode {
                SplMode::Meter => "SPL meter",
                SplMode::Leq => "Leq windows",
                SplMode::MeterLeq => "Meter + Leq",
                SplMode::Bands => "Band meter",
            },
            _ => "Transfer",
        }
    }
}

pub fn reset_axes(view: &mut ViewState) {
    let (spectrum, spl, sweep) = (view.spectrum.mode, view.spl.mode, view.distortion.mode);
    *view = ViewState::default();
    view.spectrum.mode = spectrum;
    view.spl.mode = spl;
    view.distortion.mode = sweep;
}

pub fn fold_spectrograph(
    snapshot: &Snapshot,
    root: &Measurement,
    history: &mut SpectrographHistory,
) {
    if let Some(stream) = root.config.kind.stream()
        && let Some(frame) = snapshot.latest.get(&Topic::Data {
            meas: root.id,
            stream,
        })
        && let Some(id) = frame.frame.stamp.grid_id
        && let Some(grid) = snapshot.grids.get(&id)
    {
        let (scale, level, validity) = match &frame.frame.data {
            FrameData::Spec(f) => (f.meta.scale, &f.level, None),
            FrameData::Rta(f) => (f.meta.scale, &f.level, Some(f.validity.as_slice())),
            _ => return,
        };
        if !root.running || frame.stale {
            history.mark_break();
        }
        history.push(&SpectrographFrame {
            seq: frame.frame.stamp.seq,
            at: frame.frame.stamp.capture_wall_ns,
            grid: id,
            edges: &ac2_scene::grid::column_edges(grid),
            scale,
            level,
            validity,
        });
    }
}

pub fn spl(
    snapshot: &Snapshot,
    root: &Measurement,
    status: &Status,
    view: &ViewState,
    size: Viewport,
    held: &mut Option<ac2_scene::spl::SplHold>,
) -> Option<ac2_plot::Scene> {
    use ac2_scene::view::SplMode;
    let MeasKind::Spl { config } = &root.config.kind else {
        return None;
    };
    let theme = Theme::dark();
    let get = |stream| {
        snapshot.latest.get(&Topic::Data {
            meas: root.id,
            stream,
        })
    };
    let meter = get(Stream::Spl).and_then(|f| {
        crate::panes::meter(
            &f.frame,
            crate::group_panes::freshness(snapshot, root, f),
            held,
        )
    });
    if view.spl.mode == SplMode::Meter {
        return meter.map(|m| ac2_scene::spl::spl_scene(&m, true, status, &theme, size).scene);
    }
    let stale =
        |frame: &ac2_client::TopicFrame| match crate::group_panes::freshness(snapshot, root, frame)
        {
            Freshness::Fresh { .. } => None,
            Freshness::Stopped { .. } => Some("STOPPED".into()),
            Freshness::Stale { age_s } => Some(format!("STALE {}", ac2_scene::format::age(age_s))),
            Freshness::AudioStopped { .. } => Some("AUDIO STOPPED".into()),
        };
    if view.spl.mode == SplMode::Bands {
        let frame = get(Stream::BandLeq)?;
        let FrameData::BandLeq(data) = &frame.frame.data else {
            return None;
        };
        let bands = config.bands.as_ref()?;
        let v = ac2_scene::band_leq::BandLeqView {
            meter: root.config.name.clone(),
            cal: ac2_scene::spl::cal_text(
                data.meta.cal,
                data.meta.mic_curve,
                frame.frame.stamp.capture_wall_ns,
                ac2_scene::time::ClockOffset(0),
            ),
            text: ac2_scene::band_leq::band_leq_text(bands, data),
            stale: stale(frame),
        };
        return Some(ac2_scene::band_leq::band_leq_scene(&v, status, &theme, size).scene);
    }
    let leq = get(Stream::Leq).and_then(|frame| {
        let FrameData::Leq(data) = &frame.frame.data else {
            return None;
        };
        Some(ac2_scene::leq::LeqView {
            meter: root.config.name.clone(),
            cal: ac2_scene::spl::cal_text(
                data.meta.cal,
                data.meta.mic_curve,
                frame.frame.stamp.capture_wall_ns,
                ac2_scene::time::ClockOffset(0),
            ),
            cfg: &config.leq,
            tiles: ac2_scene::leq::leq_tiles(&config.leq, data),
            history: None,
            stale: stale(frame),
            scale: data.meta.scale,
            layout: view.spl.layout,
            run: data
                .meta
                .run
                .map(|r| ac2_scene::leq::run_text(&r, &config.leq, |_| 0)),
            stage: true,
        })
    });
    match (view.spl.mode, meter, leq) {
        (SplMode::MeterLeq, Some(meter), Some(leq)) => Some(
            ac2_scene::meter_leq::meter_leq_scene(&meter, &leq, status, &theme, size)
                .leq
                .scene,
        ),
        (_, _, Some(leq)) => Some(ac2_scene::leq::leq_scene(&leq, status, &theme, size).scene),
        (SplMode::MeterLeq, Some(meter), None) => {
            Some(ac2_scene::spl::spl_scene(&meter, true, status, &theme, size).scene)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac2_proto::samples;
    use ac2_scene::view::{SpectrumMode, SplMode, SweepMode};
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    #[test]
    fn tap_cycles_match_host_g_and_reset_preserves_modes() {
        let spectrum = samples::commands()
            .into_iter()
            .find_map(|c| match c {
                ac2_proto::Command::MeasUpdate { config, .. } if config.kind.publishes_levels() => {
                    Some(config.kind)
                }
                _ => None,
            })
            .unwrap();
        let mut view = ViewState::default();
        for mode in [
            SpectrumMode::Split,
            SpectrumMode::Spectrograph,
            SpectrumMode::Spectrum,
        ] {
            cycle(&mut view, &spectrum);
            assert_eq!(view.spectrum.mode, mode);
        }
        let spl = samples::spl_measurement();
        view.spl.mode = SplMode::Meter;
        for mode in [
            SplMode::Leq,
            SplMode::MeterLeq,
            SplMode::Bands,
            SplMode::Meter,
        ] {
            cycle(&mut view, &spl.config.kind);
            assert_eq!(view.spl.mode, mode);
        }
        let sweep = samples::sweep_measurement();
        for mode in [SweepMode::Ir, SweepMode::Room, SweepMode::Response] {
            cycle(&mut view, &sweep.config.kind);
            assert_eq!(view.distortion.mode, mode);
        }
        cycle(&mut view, &spectrum);
        cycle(&mut view, &sweep.config.kind);
        view.freq = view.freq.zoom(1000.0, 4.0);
        reset_axes(&mut view);
        assert_eq!(view.spectrum.mode, SpectrumMode::Split);
        assert_eq!(view.distortion.mode, SweepMode::Ir);
        assert_eq!(view.freq, ViewState::default().freq);
    }

    #[test]
    fn spl_modes_render_their_meter_leq_and_band_data() {
        let root = samples::spl_measurement();
        let mut snapshot = Snapshot::default();
        for frame in samples::frames() {
            let stream = match frame.data {
                FrameData::Spl(_) => Stream::Spl,
                FrameData::Leq(_) => Stream::Leq,
                FrameData::BandLeq(_) => Stream::BandLeq,
                _ => continue,
            };
            let topic = Topic::Data {
                meas: root.id,
                stream,
            };
            snapshot.latest.frames.insert(
                topic.to_string().into(),
                ac2_client::TopicFrame {
                    topic,
                    frame: Arc::new(frame),
                    received: Instant::now(),
                    since_new: Duration::ZERO,
                    age: Some(0.0),
                    stale: false,
                },
            );
        }
        let mut view = ViewState::default();
        let mut held = None;
        for mode in [
            SplMode::Meter,
            SplMode::Leq,
            SplMode::MeterLeq,
            SplMode::Bands,
        ] {
            view.spl.mode = mode;
            let scene = spl(
                &snapshot,
                &root,
                &Status::default(),
                &view,
                Viewport {
                    width: 900.0,
                    height: 400.0,
                },
                &mut held,
            )
            .unwrap();
            assert!(!scene.layers.is_empty(), "empty {mode:?}");
        }
    }

    #[test]
    fn sweep_response_ir_and_room_are_different_scenes() {
        let data = samples::replies()
            .into_iter()
            .find_map(|r| match r {
                Ok(ac2_proto::ReplyBody::TraceData(data)) if data.sweep.is_some() => Some(data),
                _ => None,
            })
            .unwrap();
        let grids = samples::grids();
        let grid = grids.iter().find(|g| g.id() == data.meta.grid_id).unwrap();
        let mut view = ViewState::default();
        let size = Viewport {
            width: 900.0,
            height: 400.0,
        };
        let response = crate::panes::sweep(&data, grid, &Status::default(), &view, size);
        view.distortion.mode = SweepMode::Ir;
        let ir = crate::panes::sweep(&data, grid, &Status::default(), &view, size);
        view.distortion.mode = SweepMode::Room;
        let room = crate::panes::sweep(&data, grid, &Status::default(), &view, size);
        assert_ne!(response, ir);
        assert_ne!(ir, room);
    }
}
