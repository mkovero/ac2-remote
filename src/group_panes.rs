use crate::{groups, link::Snapshot};
use ac2_proto::{
    FrameData, Topic,
    model::{LevelScale, Measurement, TraceKind},
};
use ac2_scene::{
    Theme, ViewState, Viewport,
    banner::Status,
    spectrum::{Quantity, SpectrumTrace},
    time::Freshness,
    trace::{DisplayCache, TfTrace, TraceKey},
};

pub(crate) fn freshness(
    snapshot: &Snapshot,
    measurement: &Measurement,
    frame: &ac2_client::TopicFrame,
) -> Freshness {
    let age = frame
        .age
        .unwrap_or(frame.since_new.as_secs_f64())
        .max(frame.since_new.as_secs_f64())
        + snapshot.updated.map_or(0.0, |t| t.elapsed().as_secs_f64());
    let responding = snapshot.error.is_none()
        && snapshot
            .mirror
            .as_ref()
            .is_some_and(|m| m.responding(std::time::Instant::now()) && m.synced());
    if !responding
        || (measurement.running
            && (frame.stale || age > ac2_client::data::stale_after(&frame.topic).as_secs_f64()))
    {
        Freshness::Stale { age_s: age }
    } else if measurement.running {
        Freshness::Fresh { age_s: age }
    } else {
        Freshness::Stopped { age_s: age }
    }
}

pub fn scale(snapshot: &Snapshot, root: &Measurement) -> Option<LevelScale> {
    if !root.config.kind.publishes_levels() {
        return None;
    }
    let mut scales = Vec::new();
    for m in snapshot
        .measurements()
        .iter()
        .filter(|m| groups::live(root, m))
    {
        if let Some(stream) = m.config.kind.stream()
            && let Some(frame) = snapshot.latest.get(&Topic::Data { meas: m.id, stream })
        {
            match &frame.frame.data {
                FrameData::Spec(f) => scales.push(f.meta.scale),
                FrameData::Rta(f) => scales.push(f.meta.scale),
                _ => {}
            }
        }
    }
    for trace in snapshot
        .traces
        .values()
        .filter(|t| groups::stored(root, &t.meta))
    {
        if let TraceKind::Spectrum { scale } | TraceKind::Rta { scale } = trace.meta.kind {
            scales.push(scale);
        }
    }
    Some(
        if !scales.is_empty() && scales.iter().all(|s| *s == LevelScale::DbSpl) {
            LevelScale::DbSpl
        } else {
            LevelScale::Dbfs
        },
    )
}

/// Frame the shown spectrum curves as the desktop's Shift+Home does.
/// Return false until finite data and its grid have arrived.
pub fn fit_spectrum(snapshot: &Snapshot, root: &Measurement, view: &mut ViewState) -> bool {
    if !root.config.kind.publishes_levels() {
        return false;
    }
    let freq = ac2_scene::view::FreqRange::default();
    let mut values = Vec::new();
    let mut add = |grid: &ac2_proto::GridDef, levels: &[f32], offset: f64| {
        values.extend(
            ac2_scene::grid::column_frequencies(grid)
                .into_iter()
                .zip(levels)
                .filter(|(f, _)| *f >= freq.lo && *f <= freq.hi)
                .map(|(_, value)| f64::from(*value) + offset),
        );
    };
    for m in snapshot
        .measurements()
        .iter()
        .filter(|m| groups::live(root, m))
    {
        if let Some(stream) = m.config.kind.stream()
            && let Some(frame) = snapshot.latest.get(&Topic::Data { meas: m.id, stream })
            && let Some(grid) = frame
                .frame
                .stamp
                .grid_id
                .and_then(|id| snapshot.grids.get(&id))
        {
            match &frame.frame.data {
                FrameData::Spec(data) => add(grid, &data.level, 0.0),
                FrameData::Rta(data) => add(grid, &data.level, 0.0),
                _ => {}
            }
        }
    }
    for trace in snapshot
        .traces
        .values()
        .filter(|t| groups::stored(root, &t.meta))
    {
        if let Some(grid) = snapshot.grids.get(&trace.meta.grid_id) {
            add(grid, &trace.mag_db, trace.meta.edit.offset.0);
        }
    }
    let Some(range) = ac2_scene::view::level::fit(values) else {
        return false;
    };
    view.freq = freq;
    *view
        .spectrum
        .range_mut(scale(snapshot, root).unwrap_or(LevelScale::Dbfs)) = range;
    true
}

pub fn scene(
    snapshot: &Snapshot,
    root: &Measurement,
    status: &Status,
    view: &ViewState,
    cache: &DisplayCache,
    size: Viewport,
    history: Option<&ac2_scene::spectrograph::SpectrographHistory>,
) -> Option<ac2_plot::Scene> {
    let theme = Theme::dark();
    let measurements = snapshot.measurements();
    let refs: Vec<_> = measurements.iter().collect();
    let metas: Vec<_> = snapshot
        .mirror
        .as_ref()
        .and_then(|m| m.state.as_ref())
        .map(|s| s.traces.iter().collect())
        .unwrap_or_default();
    let colors = ac2_scene::families::curve_colours(&theme, &refs, &metas);
    let mut live = Vec::new();
    for m in measurements.iter().filter(|m| groups::live(root, m)) {
        let stream = m.config.kind.stream()?;
        if let Some(frame) = snapshot.latest.get(&Topic::Data { meas: m.id, stream })
            && let Some(grid) = frame
                .frame
                .stamp
                .grid_id
                .and_then(|id| snapshot.grids.get(&id))
        {
            live.push((
                m,
                frame,
                grid,
                ac2_scene::grid::column_frequencies(grid),
                ac2_scene::grid::column_edges(grid),
            ));
        }
    }
    live.sort_by_key(|(m, ..)| m.id != root.id);
    let stored: Vec<_> = ac2_scene::meas_list::trace_order(&refs, &metas)
        .into_iter()
        .filter(|meta| groups::stored(root, meta))
        .filter_map(|meta| snapshot.traces.get(&meta.id))
        .filter_map(|data| {
            snapshot.grids.get(&data.meta.grid_id).map(|grid| {
                (
                    data,
                    grid,
                    ac2_scene::grid::column_frequencies(grid),
                    ac2_scene::grid::column_edges(grid),
                )
            })
        })
        .collect();
    if live.is_empty() && stored.is_empty() {
        return None;
    }
    if root.config.kind.publishes_tf() {
        let mut traces: Vec<_> = live
            .iter()
            .filter_map(|(m, frame, _, freqs, _)| {
                if let FrameData::Tf(data) = &frame.frame.data {
                    Some(TfTrace::live(
                        data,
                        &frame.frame.stamp,
                        freqs,
                        &m.config.name,
                        colors.meas(m.id),
                        freshness(snapshot, m, frame),
                    ))
                } else {
                    None
                }
            })
            .collect();
        traces.extend(
            stored.iter().map(|(data, _, freqs, _)| {
                TfTrace::stored(data, freqs, colors.trace(data.meta.id))
            }),
        );
        return Some(
            ac2_scene::tf::transfer_scene(&traces, cache, status, view, &theme, size).scene,
        );
    }
    if root.config.kind.publishes_levels() {
        let mut traces: Vec<_> = live
            .iter()
            .filter_map(|(m, frame, grid, freqs, edges)| {
                let fresh = freshness(snapshot, m, frame);
                let mut trace = match &frame.frame.data {
                    FrameData::Spec(data) => SpectrumTrace::spectrum(
                        data,
                        frame.frame.stamp.capture_wall_ns,
                        None,
                        freqs,
                        edges,
                        &m.config.name,
                        colors.meas(m.id),
                        fresh,
                    ),
                    FrameData::Rta(data) => SpectrumTrace::rta(
                        data,
                        frame.frame.stamp.capture_wall_ns,
                        None,
                        freqs,
                        edges,
                        &m.config.name,
                        colors.meas(m.id),
                        fresh,
                    ),
                    _ => return None,
                };
                if matches!(frame.frame.data, FrameData::Spec(_)) {
                    trace.bin_hz = ac2_scene::grid::bin_spacing(grid);
                }
                Some(trace)
            })
            .collect();
        traces.extend(stored.iter().filter_map(|(data, grid, freqs, edges)| {
            let (scale, quantity) = match data.meta.kind {
                TraceKind::Spectrum { scale } => (
                    scale,
                    Quantity::tone(data.meta.edit.smoothing.map(|s| s.fraction)),
                ),
                TraceKind::Rta { scale } => (scale, Quantity::Band),
                _ => return None,
            };
            Some(SpectrumTrace {
                key: TraceKey::Stored(data.meta.id),
                name: data.meta.edit.name.clone(),
                color: colors.trace(data.meta.id),
                freqs,
                edges,
                level: &data.mag_db,
                validity: None,
                peak: None,
                scale,
                quantity,
                bin_hz: if quantity == Quantity::Band {
                    None
                } else {
                    ac2_scene::grid::bin_spacing(grid)
                },
                caption: "stored".into(),
                freshness: None,
                offset_db: data.meta.edit.offset.0,
                selected: false,
            })
        }));
        let mut view = *view;
        if scale(snapshot, root) == Some(LevelScale::DbSpl) {
            view.spectrum.level = view.spectrum.level_spl;
        }
        if view.spectrum.mode.spectrograph() {
            let sg = history.map(|h| ac2_scene::spectrograph::SpectrographInput {
                history: h,
                name: root.config.name.clone(),
                range: view.spectrum.range(h.scale().unwrap_or(LevelScale::Dbfs)),
                offset_db: 0.0,
                freshness: live.first().map(|(m, f, ..)| freshness(snapshot, m, f)),
            });
            return Some(
                ac2_scene::spectrograph::spectrograph_scene(
                    &traces,
                    status,
                    sg.as_ref(),
                    &view,
                    &theme,
                    size,
                )
                .scene,
            );
        }
        return Some(
            ac2_scene::spectrum::spectrum_scene(&traces, status, &view, &theme, size).scene,
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac2_proto::{
        model::{MathDomain, MeasKind, TraceOwner},
        samples,
        units::MeasId,
    };
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    fn add_frame(snapshot: &mut Snapshot, mut frame: ac2_proto::Frame, id: MeasId) {
        let stream = match &mut frame.data {
            FrameData::Spec(f) => {
                f.meas = id;
                ac2_proto::Stream::Spec
            }
            FrameData::Rta(f) => {
                f.meas = id;
                ac2_proto::Stream::Rta
            }
            _ => unreachable!(),
        };
        let topic = Topic::Data { meas: id, stream };
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

    #[test]
    fn spectrum_page_contains_fft_rta_math_and_visible_slots_together() {
        let rta = samples::commands()
            .into_iter()
            .find_map(|c| match c {
                ac2_proto::Command::MeasUpdate { config, .. }
                    if matches!(config.kind, MeasKind::Rta { .. }) =>
                {
                    Some(config)
                }
                _ => None,
            })
            .unwrap();
        let mut root = samples::state().measurements.remove(0);
        root.config.name = "FFT live".into();
        root.config.kind = MeasKind::Spectrum {
            config: ac2_proto::model::SpectrumConfig {
                input: 0,
                fft_len: 1024,
                window: ac2_proto::model::Window::Hann,
                averaging: ac2_proto::model::SpecAveraging::Exponential {
                    time_constant: ac2_proto::units::Seconds(1.0),
                },
                smoothing: None,
            },
        };
        let mut bands = root.clone();
        bands.id = MeasId(3);
        bands.config = rta;
        bands.config.name = "RTA live".into();
        let mut math = samples::math_measurement();
        math.config.name = "Math result".into();
        if let MeasKind::Math { config } = &mut math.config.kind {
            config.domain = MathDomain::Spectrum;
            config.owner = TraceOwner::Meas { meas: root.id };
        }
        let data = samples::replies()
            .into_iter()
            .find_map(|r| match r {
                Ok(ac2_proto::ReplyBody::TraceData(d)) if d.sweep.is_none() => Some(d),
                _ => None,
            })
            .unwrap();
        let mut data = *data;
        data.meta.kind = TraceKind::Spectrum {
            scale: LevelScale::Dbfs,
        };
        data.meta.edit.owner = TraceOwner::Meas { meas: root.id };
        data.meta.edit.name = "Slot capture".into();
        data.meta.edit.visible = true;
        let mut hidden = data.clone();
        hidden.mag_db.fill(500.0);
        hidden.meta.id = ac2_proto::units::TraceId(99);
        hidden.meta.edit.name = "Hidden capture".into();
        hidden.meta.edit.visible = false;
        let mut state = samples::state();
        state.measurements = vec![root.clone(), bands.clone(), math.clone()];
        state.traces = vec![data.meta.clone(), hidden.meta.clone()];
        let mut mirror = ac2_client::mirror::Mirror::new(true).view();
        mirror.state = Some(Arc::new(state));
        mirror.last_ka = Some(Instant::now());
        mirror.phase = ac2_client::mirror::Phase::Live;
        let mut snapshot = Snapshot {
            mirror: Some(Arc::new(mirror)),
            ..Default::default()
        };
        for grid in samples::grids() {
            snapshot.grids.insert(grid.id(), Arc::new(grid));
        }
        let mut fitted = ViewState {
            freq: ac2_scene::view::FreqRange {
                lo: 100.0,
                hi: 1000.0,
            },
            ..Default::default()
        };
        assert!(!fit_spectrum(&snapshot, &root, &mut fitted));
        assert_eq!(fitted.freq.lo, 100.0);
        snapshot.traces.insert(data.meta.id, Arc::new(data));
        snapshot.traces.insert(hidden.meta.id, Arc::new(hidden));
        assert!(fit_spectrum(&snapshot, &root, &mut fitted));
        assert_eq!(fitted.freq, ac2_scene::view::FreqRange::default());
        assert!(
            fitted.spectrum.level.hi < 500.0,
            "hidden curves must not affect fit"
        );
        let range = fitted.spectrum.level;
        let mut offset_snapshot = snapshot.clone();
        let trace = offset_snapshot
            .traces
            .values_mut()
            .find(|t| t.meta.edit.visible)
            .unwrap();
        Arc::make_mut(trace).meta.edit.offset.0 += 20.0;
        assert!(fit_spectrum(&offset_snapshot, &root, &mut fitted));
        assert!(fitted.spectrum.level.lo > range.lo);
        let spectrum = samples::frames()
            .into_iter()
            .find(|f| matches!(f.data, FrameData::Spec(_)))
            .unwrap();
        let rta = samples::frames()
            .into_iter()
            .find(|f| matches!(f.data, FrameData::Rta(_)))
            .unwrap();
        add_frame(&mut snapshot, spectrum.clone(), root.id);
        add_frame(&mut snapshot, spectrum, math.id);
        add_frame(&mut snapshot, rta, bands.id);
        assert_eq!(groups::roots(&snapshot.measurements()).len(), 2);
        let scene = scene(
            &snapshot,
            &root,
            &Status::default(),
            &ViewState::default(),
            &DisplayCache::default(),
            Viewport {
                width: 1200.0,
                height: 600.0,
            },
            None,
        )
        .unwrap();
        let labels: Vec<_> = scene
            .layers
            .iter()
            .flat_map(|l| &l.labels)
            .map(|l| l.text.as_str())
            .collect();
        for name in ["FFT live", "RTA live", "Math result", "Slot capture"] {
            assert!(
                labels.iter().any(|l| l.contains(name)),
                "missing {name}: {labels:?}"
            );
        }
        assert!(!labels.iter().any(|l| l.contains("Hidden capture")));
        let mut history = ac2_scene::spectrograph::SpectrographHistory::new(30);
        crate::modes::fold_spectrograph(&snapshot, &root, &mut history);
        assert!(!history.is_empty());
        let mut view = ViewState::default();
        view.spectrum.mode = ac2_scene::view::SpectrumMode::Split;
        let split = super::scene(
            &snapshot,
            &root,
            &Status::default(),
            &view,
            &DisplayCache::default(),
            Viewport {
                width: 1200.0,
                height: 600.0,
            },
            Some(&history),
        )
        .unwrap();
        assert!(split.layers.iter().any(|l| !l.heatmaps.is_empty()));
    }
}
