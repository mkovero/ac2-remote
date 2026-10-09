//! Read-only client worker. Only hello/state/grid requests and data subscriptions are used.
use ac2_client::{Client, ClientConfig, ClientError, Latest, MirrorView};
use ac2_proto::{
    GridDef, GridId, Subscription,
    model::{Measurement, TraceData, TraceKind},
    units::{MeasId, TraceId},
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Default, Clone)]
pub struct Snapshot {
    pub mirror: Option<Arc<MirrorView>>,
    pub latest: Latest,
    pub grids: BTreeMap<GridId, Arc<GridDef>>,
    pub error: Option<String>,
    pub updated: Option<Instant>,
    pub sweep: Option<Arc<TraceData>>,
    pub traces: BTreeMap<TraceId, Arc<TraceData>>,
}
impl Snapshot {
    pub fn measurements(&self) -> Vec<Measurement> {
        self.mirror
            .as_ref()
            .and_then(|m| m.state.as_ref())
            .map(|s| s.measurements.clone())
            .unwrap_or_default()
    }
    pub fn transfers(&self) -> Vec<Measurement> {
        self.measurements()
            .into_iter()
            .filter(|m| m.config.kind.publishes_tf())
            .collect()
    }
}

pub struct Link {
    pub snapshot: Arc<Mutex<Snapshot>>,
    selected: Arc<Mutex<Option<MeasId>>>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
}
impl Link {
    pub fn start(config: ClientConfig) -> Self {
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let link = Self {
            snapshot: Arc::new(Mutex::new(Snapshot::default())),
            selected: Arc::new(Mutex::new(None)),
            stop: Some(stop),
        };
        let (snapshot, selected) = (link.snapshot.clone(), link.selected.clone());
        std::thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                rt.block_on(async {
                    tokio::select! {
                        _ = stopped => Ok(()),
                        result = async {
                            loop {
                                let error = match observe(config.clone(), &snapshot, &selected).await {
                                    Ok(()) => return Ok(()),
                                    Err(error) => error,
                                };
                                if matches!(error, ClientError::VersionMismatch { .. }) {
                                    return Err(error.to_string());
                                }
                                snapshot.lock().unwrap().error = Some(format!("{error} · retrying; check host authorization and network"));
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            }
                        }
                        => result,
                    }
                })
            })();
            if let Err(error) = result {
                snapshot.lock().unwrap().error = Some(error);
            }
        });
        link
    }
    pub fn select(&self, meas: Option<MeasId>) {
        *self.selected.lock().unwrap() = meas;
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().unwrap().clone()
    }
}
impl Drop for Link {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

async fn observe(
    config: ClientConfig,
    snapshot: &Mutex<Snapshot>,
    selected: &Mutex<Option<MeasId>>,
) -> Result<(), ClientError> {
    let client = Client::connect(config).await?;
    client.wait_synced(Duration::from_secs(8)).await?;
    let mut subscribed = Vec::new();
    let mut traces: BTreeMap<TraceId, Arc<TraceData>> = BTreeMap::new();
    let mut trace_meta = BTreeMap::new();
    loop {
        let selected = *selected.lock().unwrap();
        let mirror = client.view();
        let measurement = mirror
            .state
            .as_ref()
            .and_then(|s| s.measurements.iter().find(|m| Some(m.id) == selected));
        let wanted = measurement
            .map(|root| crate::groups::topics(root, &mirror.state.as_ref().unwrap().measurements))
            .unwrap_or_default();
        if wanted != subscribed {
            for topic in &subscribed {
                if !wanted.contains(topic) {
                    client.unsubscribe(Subscription::Topic(*topic))?;
                }
            }
            for topic in &wanted {
                if !subscribed.contains(topic) {
                    client.subscribe(Subscription::Topic(*topic))?;
                }
            }
            subscribed = wanted;
        }
        let latest_sweep = mirror
            .state
            .as_ref()
            .and_then(|state| crate::groups::sweep_trace(measurement, &state.traces))
            .cloned();
        let (latest, mut grids) = client.latest_with_grids().await?;
        if let Some(state) = &mirror.state {
            let visible: Vec<_> = state
                .traces
                .iter()
                .filter(|t| t.edit.visible || t.kind == TraceKind::Sweep)
                .collect();
            traces.retain(|id, _| visible.iter().any(|t| t.id == *id));
            trace_meta.retain(|id, _| traces.contains_key(id));
            for meta in visible {
                if trace_meta.get(&meta.id) != Some(meta)
                    && let ac2_proto::ReplyBody::TraceData(data) = client
                        .call(ac2_proto::Command::TraceGet { trace: meta.id })
                        .await?
                {
                    traces.insert(meta.id, Arc::from(data));
                    trace_meta.insert(meta.id, meta.clone());
                }
            }
        }
        for data in traces.values() {
            grids.insert(data.meta.grid_id, client.grid(data.meta.grid_id).await?);
        }
        let sweep = latest_sweep.and_then(|meta| traces.get(&meta.id).cloned());
        *snapshot.lock().unwrap() = Snapshot {
            mirror: Some(client.view()),
            latest,
            grids,
            error: None,
            updated: Some(Instant::now()),
            sweep: sweep.clone(),
            traces: traces.clone(),
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
