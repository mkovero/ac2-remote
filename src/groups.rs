use ac2_proto::{
    Topic,
    model::{MeasKind, Measurement, TraceKind, TraceMeta},
};

pub fn roots(measurements: &[Measurement]) -> Vec<Measurement> {
    measurements
        .iter()
        .filter(|m| !matches!(m.config.kind, MeasKind::Math { .. }))
        .cloned()
        .collect()
}

pub fn live(root: &Measurement, candidate: &Measurement) -> bool {
    if root.config.kind.publishes_levels() {
        // The host spectrum pane combines FFT and RTA curves.
        candidate.config.kind.publishes_levels()
    } else {
        candidate.id == root.id
            || matches!(&candidate.config.kind,
            MeasKind::Math { config } if config.owner.meas() == Some(root.id)
                && Some(config.domain.stream()) == root.config.kind.stream())
    }
}

pub fn stored(root: &Measurement, trace: &TraceMeta) -> bool {
    if !trace.edit.visible {
        return false;
    }
    if root.config.kind.publishes_levels() {
        matches!(
            trace.kind,
            TraceKind::Spectrum { .. } | TraceKind::Rta { .. }
        )
    } else {
        trace.edit.owner.meas() == Some(root.id)
            && matches!(
                trace.kind,
                TraceKind::Transfer | TraceKind::Target | TraceKind::Sweep
            )
    }
}

pub fn topics(root: &Measurement, measurements: &[Measurement]) -> Vec<Topic> {
    let mut topics: Vec<_> = measurements
        .iter()
        .filter(|m| live(root, m))
        .filter_map(|m| {
            m.config
                .kind
                .stream()
                .map(|stream| Topic::Data { meas: m.id, stream })
        })
        .collect();
    if let MeasKind::Spl { config } = &root.config.kind {
        topics.push(Topic::Data {
            meas: root.id,
            stream: ac2_proto::Stream::Leq,
        });
        if config.bands.is_some() {
            topics.push(Topic::Data {
                meas: root.id,
                stream: ac2_proto::Stream::BandLeq,
            });
        }
    }
    topics
}

pub fn sweep_trace<'a>(
    root: Option<&Measurement>,
    traces: &'a [TraceMeta],
) -> Option<&'a TraceMeta> {
    let sweeps = || traces.iter().filter(|t| t.kind == TraceKind::Sweep);
    root.filter(|m| matches!(m.config.kind, MeasKind::Sweep { .. }))
        .and_then(|m| {
            sweeps()
                .filter(|t| t.edit.owner.meas() == Some(m.id))
                .max_by_key(|t| t.id)
        })
        .or_else(|| sweeps().max_by_key(|t| t.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac2_proto::{Stream, samples, units::MeasId};
    #[test]
    fn child_math_channels_do_not_become_swipe_pages() {
        let root = samples::state().measurements.remove(0);
        let child = samples::math_measurement();
        let mut other = child.clone();
        if let MeasKind::Math { config } = &mut other.config.kind {
            config.owner = ac2_proto::model::TraceOwner::Meas { meas: MeasId(99) };
        }
        other.id = MeasId(100);
        let all = vec![root.clone(), child.clone(), other];
        assert_eq!(roots(&all), vec![root.clone()]);
        let wanted = topics(&root, &all);
        assert_eq!(wanted.len(), 2);
        assert!(wanted.contains(&Topic::Data {
            meas: child.id,
            stream: Stream::Tf
        }));
    }

    #[test]
    fn distortion_uses_hidden_sweep_and_falls_back_when_measurement_has_no_run() {
        let mut root = samples::sweep_measurement();
        let trace = samples::replies()
            .into_iter()
            .find_map(|r| match r {
                Ok(ac2_proto::ReplyBody::TraceData(data)) if data.sweep.is_some() => {
                    Some(data.meta)
                }
                _ => None,
            })
            .unwrap();
        let mut hidden = trace.clone();
        hidden.edit.visible = false;
        hidden.edit.owner = ac2_proto::model::TraceOwner::Meas { meas: root.id };
        assert_eq!(
            sweep_trace(Some(&root), &[hidden.clone()]).unwrap().id,
            hidden.id
        );
        root.id = MeasId(99);
        assert_eq!(
            sweep_trace(Some(&root), &[hidden.clone()]).unwrap().id,
            hidden.id
        );
        assert!(sweep_trace(Some(&root), &[]).is_none());
    }
}
