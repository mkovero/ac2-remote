use crate::view::ViewState;
use ac2_proto::{Frame, FrameData, GridDef, model::TraceData};
use ac2_scene::{Theme, Viewport, banner::Status, time::Freshness, trace::DisplayCache};

#[allow(clippy::too_many_arguments)]
pub fn live(
    frame: &Frame,
    grid: Option<&GridDef>,
    name: &str,
    freshness: Freshness,
    status: &Status,
    view: &ViewState,
    cache: &DisplayCache,
    size: Viewport,
    held: &mut Option<ac2_scene::spl::SplHold>,
) -> Option<ac2_plot::Scene> {
    let theme = Theme::dark();
    let freqs = grid
        .map(ac2_scene::grid::column_frequencies)
        .unwrap_or_default();
    let edges = grid.map(ac2_scene::grid::column_edges).unwrap_or_default();
    match &frame.data {
        FrameData::Tf(data) if grid.is_some() => {
            let trace = ac2_scene::trace::TfTrace::live(
                data,
                &frame.stamp,
                &freqs,
                name,
                theme.traces[0],
                freshness,
            );
            Some(ac2_scene::tf::transfer_scene(&[trace], cache, status, view, &theme, size).scene)
        }
        FrameData::Spec(data) if grid.is_some() => {
            let mut view = *view;
            if data.meta.scale == ac2_proto::model::LevelScale::DbSpl {
                view.spectrum.level = view.spectrum.level_spl;
            }
            let mut trace = ac2_scene::spectrum::SpectrumTrace::spectrum(
                data,
                frame.stamp.capture_wall_ns,
                None,
                &freqs,
                &edges,
                name,
                theme.traces[0],
                freshness,
            );
            trace.bin_hz = grid.and_then(ac2_scene::grid::bin_spacing);
            Some(ac2_scene::spectrum::spectrum_scene(&[trace], status, &view, &theme, size).scene)
        }
        FrameData::Rta(data) if grid.is_some() => {
            let mut view = *view;
            if data.meta.scale == ac2_proto::model::LevelScale::DbSpl {
                view.spectrum.level = view.spectrum.level_spl;
            }
            let trace = ac2_scene::spectrum::SpectrumTrace::rta(
                data,
                frame.stamp.capture_wall_ns,
                None,
                &freqs,
                &edges,
                name,
                theme.traces[0],
                freshness,
            );
            Some(ac2_scene::spectrum::spectrum_scene(&[trace], status, &view, &theme, size).scene)
        }
        FrameData::Spl(_) => {
            let readout = meter(frame, freshness, held)?;
            Some(ac2_scene::spl::spl_scene(&readout, true, status, &theme, size).scene)
        }
        _ => None,
    }
}

pub fn meter(
    frame: &Frame,
    freshness: Freshness,
    held: &mut Option<ac2_scene::spl::SplHold>,
) -> Option<ac2_scene::spl::SplReadout> {
    let FrameData::Spl(data) = &frame.data else {
        return None;
    };
    ac2_scene::spl::SplHold::update(
        held,
        data,
        frame.stamp.capture_wall_ns.0,
        frame.stamp.config_rev,
        ac2_scene::spl::display_period_s(data.meta.time_weighting),
    );
    let displayed = &held
        .as_ref()
        .expect("first SPL frame initializes the hold")
        .frame;
    let cal = ac2_scene::spl::cal_text(
        data.meta.cal,
        data.meta.mic_curve,
        frame.stamp.capture_wall_ns,
        ac2_scene::time::ClockOffset(0),
    );
    let start = ac2_proto::units::WallNs(
        frame
            .stamp
            .capture_wall_ns
            .0
            .saturating_sub((data.meta.duration.0.max(0.0) * 1e9).round() as u64),
    );
    let since = ac2_scene::spl::meter_since(start, frame.stamp.capture_wall_ns, |_| 0, None);
    let readout =
        ac2_scene::spl::spl_readout(displayed, data.meta.level, cal, Some(freshness), since);
    Some(readout)
}

pub fn sweep(
    data: &TraceData,
    grid: &GridDef,
    status: &Status,
    view: &ViewState,
    size: Viewport,
) -> ac2_plot::Scene {
    let theme = Theme::dark();
    match view.sweep_mode {
        ac2_scene::view::SweepMode::Room => {
            return ac2_scene::room::room_scene(
                data.sweep.as_ref().and_then(|s| s.room.as_ref()),
                Some(&data.meta.edit.name),
                status,
                &theme,
                size,
            )
            .scene;
        }
        ac2_scene::view::SweepMode::Ir => {
            if let Some(scene) = ac2_scene::distortion::sweep_ir_scene(
                data,
                theme.traces[0],
                status,
                view,
                &theme,
                size,
            ) {
                return scene.scene;
            }
        }
        _ => {}
    }
    let freqs = ac2_scene::grid::column_frequencies(grid);
    ac2_scene::distortion::distortion_scene(
        Some(ac2_scene::distortion::SweepView {
            data,
            freqs: &freqs,
            color: theme.traces[0],
        }),
        status,
        view,
        &theme,
        size,
    )
    .scene
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_live_types_and_stored_sweep_render_in_landscape() {
        let grids = ac2_proto::samples::grids();
        let size = Viewport {
            width: 900.0,
            height: 400.0,
        };
        let mut count = 0;
        for frame in ac2_proto::samples::frames().into_iter().filter(|f| {
            matches!(
                f.data,
                FrameData::Tf(_) | FrameData::Spec(_) | FrameData::Rta(_) | FrameData::Spl(_)
            )
        }) {
            let grid = frame
                .stamp
                .grid_id
                .and_then(|id| grids.iter().find(|g| g.id() == id));
            let scene = live(
                &frame,
                grid,
                "Measurement",
                Freshness::Fresh { age_s: 0.0 },
                &Status::default(),
                &ViewState::default(),
                &DisplayCache::default(),
                size,
                &mut None,
            )
            .unwrap();
            assert!(!scene.layers.is_empty());
            assert_eq!(scene.viewport.width, 900.0);
            count += 1;
        }
        assert_eq!(count, 4);
        let data = ac2_proto::samples::replies()
            .into_iter()
            .find_map(|r| match r {
                Ok(ac2_proto::ReplyBody::TraceData(data)) if data.sweep.is_some() => Some(data),
                _ => None,
            })
            .unwrap();
        let grid = grids.iter().find(|g| g.id() == data.meta.grid_id).unwrap();
        let scene = sweep(&data, grid, &Status::default(), &ViewState::default(), size);
        assert!(!scene.layers.is_empty());
    }

    #[test]
    fn calibrated_rta_uses_spl_range_and_preserves_dbfs_range() {
        let frame = ac2_proto::samples::frames()
            .into_iter()
            .find(|f| matches!(f.data, FrameData::Rta(_)))
            .unwrap();
        let grids = ac2_proto::samples::grids();
        let grid = frame
            .stamp
            .grid_id
            .and_then(|id| grids.iter().find(|g| g.id() == id));
        let view = ViewState::default();
        let scene = live(
            &frame,
            grid,
            "RTA",
            Freshness::Fresh { age_s: 0.0 },
            &Status::default(),
            &view,
            &DisplayCache::default(),
            Viewport {
                width: 900.0,
                height: 400.0,
            },
            &mut None,
        )
        .unwrap();
        let labels: Vec<_> = scene
            .layers
            .iter()
            .flat_map(|l| &l.labels)
            .map(|l| l.text.as_str())
            .collect();
        assert!(labels.contains(&"120"));
        assert!(!labels.contains(&"-100"));
        assert_eq!(view.spectrum.level, ViewState::default().spectrum.level);
    }

    #[test]
    fn spl_number_holds_between_display_updates() {
        let mut frame = ac2_proto::samples::frames()
            .into_iter()
            .find(|f| matches!(f.data, FrameData::Spl(_)))
            .unwrap();
        if let FrameData::Spl(data) = &mut frame.data {
            data.meta.time_weighting = ac2_proto::model::TimeWeighting::Fast;
            data.meta.level = 94.1;
        }
        let mut held = None;
        let mut render = |frame: &Frame| {
            live(
                frame,
                None,
                "SPL",
                Freshness::Fresh { age_s: 0.0 },
                &Status::default(),
                &ViewState::default(),
                &DisplayCache::default(),
                Viewport {
                    width: 900.0,
                    height: 400.0,
                },
                &mut held,
            )
            .unwrap()
        };
        let number = |scene: &ac2_plot::Scene, text: &str| {
            scene
                .layers
                .iter()
                .flat_map(|l| &l.labels)
                .any(|l| l.size > 40.0 && l.text == text)
        };
        assert!(number(&render(&frame), "94.1"));
        frame.stamp.capture_wall_ns.0 += 100_000_000;
        if let FrameData::Spl(data) = &mut frame.data {
            data.meta.level = 70.0;
        }
        assert!(number(&render(&frame), "94.1"));
        frame.stamp.capture_wall_ns.0 += 400_000_000;
        assert!(number(&render(&frame), "70.0"));
    }
}
