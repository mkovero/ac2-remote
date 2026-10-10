use crate::view::ViewState;
use crate::{discovery::Discovery, link::Link, pairing::Connection, plot};
use ac2_client::{ClientConfig, Endpoints, KeyDir, RemoteAddr};
use ac2_proto::{FrameData, Topic, units::MeasId};
use ac2_scene::{
    Viewport,
    banner::{Status, no_delay_estimate},
    time::Freshness,
    trace::DisplayCache,
};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct RemoteApp {
    keys: KeyDir,
    host: String,
    server_key: String,
    verify: bool,
    error: Option<String>,
    link: Option<Link>,
    selected: Option<MeasId>,
    shown: Option<MeasId>,
    fit_spectrum_pending: bool,
    view: ViewState,
    cache: DisplayCache,
    demo: bool,
    discovery: Option<Discovery>,
    connection_path: PathBuf,
    demo_index: usize,
    swipe: crate::gestures::Swipe,
    measurement_step: i32,
    spl_hold: Option<ac2_scene::spl::SplHold>,
    tapped: bool,
    tap_at: Option<Instant>,
    spectrograph: Option<(MeasId, ac2_scene::spectrograph::SpectrographHistory)>,
}
impl RemoteApp {
    pub fn new(data_dir: PathBuf, demo: bool) -> Self {
        let keys = KeyDir::new(data_dir.join("keys"));
        let connection_path = data_dir.join("connection.json");
        let (saved, error) = match Connection::load(&connection_path) {
            Ok(saved) => (saved, None),
            Err(error) => (None, Some(format!("Cannot read saved connection: {error}"))),
        };
        let verify = saved
            .as_ref()
            .is_some_and(|connection| connection.trusted(&keys));
        Self {
            keys,
            host: saved.as_ref().map_or_else(String::new, |c| c.host.clone()),
            server_key: saved
                .as_ref()
                .map_or_else(String::new, |c| c.server_key.clone()),
            verify,
            error,
            link: None,
            selected: None,
            shown: None,
            fit_spectrum_pending: false,
            view: ViewState::default(),
            cache: DisplayCache::default(),
            demo,
            discovery: (!demo).then(Discovery::start),
            connection_path,
            demo_index: std::env::var("AC2_REMOTE_DEMO_INDEX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            swipe: Default::default(),
            measurement_step: 0,
            spl_hold: None,
            tapped: false,
            tap_at: None,
            spectrograph: None,
        }
    }
    fn connection_ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("Preview sample plot").clicked() {
            self.demo = true;
            self.discovery = None;
        }
        ui.heading("Find a rig");
        ui.label("Use the same Wi-Fi/LAN as the daemon host.");
        if ui.button("Scan again").clicked() {
            self.discovery = Some(Discovery::start());
        }
        if let Some(discovery) = &self.discovery {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
            let found = discovery.snapshot();
            if let Some(error) = found.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            if found.rigs.is_empty() {
                ui.label("Looking for ac2 rigs… Manual connection is available below.");
            }
            if found.invalid > 0 {
                ui.label("Some rigs use an unsupported discovery record. Update ac2d for automatic pairing.");
            }
            for rig in found.rigs {
                let title = format!("{} · {}", rig.advert.name, rig.connect_host());
                if ui.button(title).clicked() {
                    match Connection::from_rig(&rig) {
                        Ok(connection) => {
                            self.verify = connection.trusted(&self.keys);
                            let changed = connection
                                .host
                                .parse::<RemoteAddr>()
                                .ok()
                                .and_then(|addr| self.keys.server_key(&addr.host).ok())
                                .is_some_and(|pinned| pinned.to_z85() != connection.server_key);
                            self.host = connection.host;
                            self.server_key = connection.server_key;
                            self.error = changed.then(|| "This host advertises a different server key. Verify its new fingerprint on the daemon host.".into());
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
            }
        }
        if !self.host.is_empty() {
            ui.separator();
            ui.label(format!("Selected rig: {}", self.host));
        }
        if let Ok(key) = ac2_zmq::PublicKey::from_z85(self.server_key.trim()) {
            ui.monospace(format!(
                "Server fingerprint: {}",
                ac2_client::fingerprint(&key)
            ));
            if !self.verify {
                ui.label("Compare this fingerprint with the daemon host's Connection settings.");
                ui.checkbox(&mut self.verify, "The server fingerprint matches");
            } else {
                ui.label("Server fingerprint verified");
            }
        }
        match self.keys.ensure_client_keypair() {
            Ok((kp, _)) => {
                ui.monospace(format!(
                    "Phone fingerprint: {}",
                    ac2_client::fingerprint(&kp.public)
                ));
                ui.label("Tap Connect, then authorize this fingerprint under Refused keys in the host's Connection settings. No keys to type.");
            }
            Err(error) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
            }
        }
        if ui
            .add_enabled(
                self.verify && !self.host.is_empty(),
                egui::Button::new("Connect"),
            )
            .clicked()
        {
            self.error = self.connect().err();
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        ui.collapsing("Manual connection", |ui| {
            ui.label("Daemon address (host or host:port)");
            if ui.text_edit_singleline(&mut self.host).changed() {
                self.verify = false;
            }
            ui.label("Server public key (40-character Z85)");
            if ui.text_edit_singleline(&mut self.server_key).changed() {
                self.verify = false;
            }
            if let Ok((kp, _)) = self.keys.ensure_client_keypair() {
                ui.label(
                    "For hosts without an authorization UI, add this line to authorized_clients:",
                );
                ui.monospace(format!("ac2-remote {}", kp.public.to_z85()));
            }
        });
        ui.label("Start and configure measurements on the host. This viewer does not change the session or stimulus.");
    }
    fn plot_area(
        &mut self,
        ui: &mut egui::Ui,
        scale: Option<ac2_proto::model::LevelScale>,
    ) -> egui::Rect {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(egui::vec2(1.0, 1.0)),
            egui::Sense::click_and_drag(),
        );
        self.swipe(ui, &response);
        self.tapped = false;
        if self.swipe.pinched() || response.dragged() {
            self.tap_at = None;
        }
        if response.double_clicked() {
            self.tap_at = None;
            crate::modes::reset_axes(&mut self.view);
        } else if response.clicked() && !self.swipe.pinched() {
            self.tap_at = Some(Instant::now());
        }
        if let Some(at) = self.tap_at {
            let delay = Duration::from_secs_f64(
                ui.ctx().options(|o| o.input_options.max_double_click_delay),
            );
            if at.elapsed() >= delay {
                self.tapped = true;
                self.tap_at = None;
            } else {
                ui.ctx()
                    .request_repaint_after(delay.saturating_sub(at.elapsed()));
            }
        }
        if let Some(touch) = ui
            .input(|i| i.multi_touch())
            .filter(|t| rect.contains(t.start_pos))
        {
            let x = ((touch.center_pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let anchor =
                self.view.freq.lo * (self.view.freq.hi / self.view.freq.lo).powf(f64::from(x));
            self.view.freq = self
                .view
                .freq
                .zoom(anchor, f64::from(touch.zoom_delta_2d.x));
            let level = match scale {
                Some(ac2_proto::model::LevelScale::Dbfs) => &mut self.view.spectrum.level,
                Some(ac2_proto::model::LevelScale::DbSpl) => &mut self.view.spectrum.level_spl,
                None => &mut self.view.tf.magnitude_db,
            };
            *level = ac2_scene::view::level::zoom(
                *level,
                (level.lo + level.hi) / 2.0,
                f64::from(touch.zoom_delta_2d.y),
            );
            let octaves = (self.view.freq.hi / self.view.freq.lo).log2();
            self.view.freq = self
                .view
                .freq
                .pan(-f64::from(touch.translation_delta.x / rect.width()) * octaves);
        }
        rect
    }
    fn message(ui: &egui::Ui, rect: egui::Rect, text: &str) {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(18.0),
            egui::Color32::LIGHT_GRAY,
        );
    }
    fn swipe(&mut self, ui: &egui::Ui, response: &egui::Response) {
        let step = ui.input(|i| {
            self.swipe
                .update(&i.events, response.rect, i.multi_touch().is_some())
        });
        if step != 0 {
            self.measurement_step = step;
        }
    }
    fn step_measurement(&mut self, measurements: &[ac2_proto::model::Measurement]) {
        if self.measurement_step != 0 {
            if let Some(index) = measurements
                .iter()
                .position(|m| Some(m.id) == self.selected)
            {
                let next = (index as i32 + self.measurement_step)
                    .rem_euclid(measurements.len() as i32) as usize;
                self.selected = Some(measurements[next].id);
            }
            self.measurement_step = 0;
        }
    }
    fn connect(&mut self) -> Result<(), String> {
        let addr: RemoteAddr = self
            .host
            .trim()
            .parse::<RemoteAddr>()
            .map_err(|e| e.to_string())?;
        // Replacement pins require explicit fingerprint verification on every connection.
        let key =
            ac2_zmq::PublicKey::from_z85(self.server_key.trim()).map_err(|e| e.to_string())?;
        if !self.verify {
            return Err(
                "Compare the server fingerprint on the daemon host, then confirm it below.".into(),
            );
        }
        self.keys
            .ensure_client_keypair()
            .map_err(|e| e.to_string())?;
        self.keys
            .pin_server(&addr.host, key)
            .map_err(|e| e.to_string())?;
        Connection {
            host: self.host.trim().to_owned(),
            server_key: key.to_z85(),
        }
        .save(&self.connection_path)?;
        let mut config = ClientConfig::new(Endpoints::remote(&addr), "ac2-remote");
        config.curve = Some(
            self.keys
                .curve_client(&addr.host)
                .map_err(|e| e.to_string())?,
        );
        self.link = Some(Link::start(config));
        self.discovery = None;
        self.selected = None;
        self.shown = None;
        Ok(())
    }
}
fn frame_scale(data: &FrameData) -> Option<ac2_proto::model::LevelScale> {
    match data {
        FrameData::Spec(f) => Some(f.meta.scale),
        FrameData::Rta(f) => Some(f.meta.scale),
        _ => None,
    }
}
impl eframe::App for RemoteApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.spacing_mut().interact_size.y = 44.0;
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.spl_hold = None;
            self.spectrograph = None;
            self.demo = false;
            self.link = None;
            self.discovery = Some(Discovery::start());
            return;
        }
        if self.demo {
            let frames: Vec<_> = ac2_proto::samples::frames()
                .into_iter()
                .filter(|f| {
                    matches!(
                        f.data,
                        FrameData::Tf(_)
                            | FrameData::Spec(_)
                            | FrameData::Rta(_)
                            | FrameData::Spl(_)
                    )
                })
                .collect();
            let count = frames.len() + 1;
            self.demo_index %= count;
            let frame = frames.get(self.demo_index);
            let name = frame.map_or("Sweep", |f| match &f.data {
                FrameData::Tf(_) => "Transfer",
                FrameData::Spec(_) => "Spectrum",
                FrameData::Rta(_) => "RTA",
                FrameData::Spl(_) => "SPL",
                _ => "",
            });
            ui.label(format!("DEMO · {} / {count} · {name}", self.demo_index + 1));
            let rect = self.plot_area(ui, frame.and_then(|f| frame_scale(&f.data)));
            let size = Viewport {
                width: rect.width(),
                height: rect.height(),
            };
            let grids = ac2_proto::samples::grids();
            let scene = if let Some(frame) = frame {
                let grid = frame
                    .stamp
                    .grid_id
                    .and_then(|id| grids.iter().find(|g| g.id() == id));
                crate::panes::live(
                    frame,
                    grid,
                    "Sample measurement",
                    Freshness::Stopped { age_s: 0.0 },
                    &Status::default(),
                    &self.view,
                    &self.cache,
                    size,
                    &mut self.spl_hold,
                )
            } else {
                ac2_proto::samples::replies()
                    .into_iter()
                    .find_map(|r| match r {
                        Ok(ac2_proto::ReplyBody::TraceData(data)) if data.sweep.is_some() => grids
                            .iter()
                            .find(|g| g.id() == data.meta.grid_id)
                            .map(|grid| {
                                crate::panes::sweep(
                                    &data,
                                    grid,
                                    &Status::default(),
                                    &self.view,
                                    size,
                                )
                            }),
                        _ => None,
                    })
            };
            if let Some(scene) = scene {
                plot::paint(ui, plot::PlotSlot(0), rect, Arc::new(scene));
            }
            if self.measurement_step != 0 {
                self.demo_index = (self.demo_index as i32 + self.measurement_step)
                    .rem_euclid(count as i32) as usize;
                self.measurement_step = 0;
            }
            return;
        }
        if self.link.is_none() {
            ui.heading("ac2 Remote");
            egui::ScrollArea::vertical().show(ui, |ui| self.connection_ui(ui));
            return;
        }
        ui.ctx().request_repaint_after(Duration::from_millis(50));
        let snapshot = self.link.as_ref().unwrap().snapshot();
        let responding = snapshot.error.is_none()
            && snapshot
                .mirror
                .as_ref()
                .is_some_and(|m| m.responding(Instant::now()) && m.synced());
        let measurements = crate::groups::roots(&snapshot.measurements());
        if !measurements.iter().any(|m| Some(m.id) == self.selected) {
            self.selected = measurements.first().map(|m| m.id);
        }
        self.link.as_ref().unwrap().select(self.selected);
        let Some(meas) = measurements.iter().find(|m| Some(m.id) == self.selected) else {
            ui.label(if responding {
                "No measurements on this rig"
            } else {
                "Connecting / waiting for host authorization"
            });
            let rect = self.plot_area(ui, None);
            Self::message(ui, rect, "Create measurements on the host");
            return;
        };
        if self.shown != Some(meas.id) {
            self.shown = Some(meas.id);
            self.fit_spectrum_pending = meas.config.kind.publishes_levels();
        }
        if self.fit_spectrum_pending
            && crate::group_panes::fit_spectrum(&snapshot, meas, &mut self.view)
        {
            self.fit_spectrum_pending = false;
        }
        let position = measurements.iter().position(|m| m.id == meas.id).unwrap() + 1;
        let topic = meas.config.kind.stream().map(|stream| Topic::Data {
            meas: meas.id,
            stream,
        });
        let current = topic.and_then(|topic| snapshot.latest.get(&topic));
        let elapsed = snapshot.updated.map_or(0.0, |t| t.elapsed().as_secs_f64());
        let age = current.map_or(0.0, |f| {
            f.age
                .unwrap_or(f.since_new.as_secs_f64())
                .max(f.since_new.as_secs_f64())
                + elapsed
        });
        let stale = !responding || (meas.running && current.is_some_and(|f| f.stale || age > 1.0));
        let freshness = if stale {
            Freshness::Stale { age_s: age }
        } else if !meas.running {
            Freshness::Stopped { age_s: age }
        } else {
            Freshness::Fresh { age_s: age }
        };
        ui.label(format!(
            "{} · {} · {position}/{} · {}{}",
            meas.config.name,
            crate::modes::name(&self.view, &meas.config.kind),
            measurements.len(),
            if meas.running { "Running" } else { "Stopped" },
            if stale { " · STALE" } else { "" }
        ));
        if let Some(error) = &snapshot.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if topic.is_none()
            && let Some(sweep) = &snapshot.sweep
        {
            ui.label(format!("Result: {}", sweep.meta.edit.name));
        }
        let rect = self.plot_area(ui, crate::group_panes::scale(&snapshot, meas));
        if self.tapped || ui.input(|i| i.key_pressed(egui::Key::G)) {
            crate::modes::cycle(&mut self.view, &meas.config.kind);
        }
        if meas.config.kind.publishes_levels() && self.view.spectrum.mode.spectrograph() {
            if self
                .spectrograph
                .as_ref()
                .is_none_or(|(id, _)| *id != meas.id)
            {
                self.spectrograph = Some((
                    meas.id,
                    ac2_scene::spectrograph::SpectrographHistory::new(
                        self.view.spectrum.spectrograph.span_s,
                    ),
                ));
            }
            if let Some((_, history)) = &mut self.spectrograph {
                crate::modes::fold_spectrograph(&snapshot, meas, history);
            }
        } else {
            self.spectrograph = None;
        }
        let size = Viewport {
            width: rect.width(),
            height: rect.height(),
        };
        let daemon_now = ac2_proto::units::WallNs(
            (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as i128
                + snapshot
                    .mirror
                    .as_ref()
                    .and_then(|m| m.clock_offset_ns)
                    .unwrap_or(0))
            .clamp(0, u64::MAX as i128) as u64,
        );
        let status = Status {
            daemon_silence_s: snapshot
                .mirror
                .as_ref()
                .and_then(|m| m.last_ka)
                .map_or(2.0, |t| t.elapsed().as_secs_f64()),
            protection: current
                .filter(|_| meas.running)
                .map(|f| f.frame.stamp.protection)
                .unwrap_or_default(),
            frame_age_s: meas.running.then_some(age),
            timing: snapshot.mirror.as_ref().and_then(|m| m.timing),
            no_delay_estimate: no_delay_estimate(meas),
            audio_stopped: snapshot
                .mirror
                .as_ref()
                .and_then(|m| m.state.as_ref())
                .and_then(|s| s.session.stopped.as_ref())
                .map(|s| ac2_scene::audio::audio_stopped_text(s, daemon_now, |_| 0)),
            ..Default::default()
        };
        let scene = if topic.is_none() {
            snapshot.sweep.as_ref().and_then(|data| {
                snapshot
                    .grids
                    .get(&data.meta.grid_id)
                    .map(|grid| crate::panes::sweep(data, grid, &status, &self.view, size))
            })
        } else if meas.config.kind.publishes_tf() || meas.config.kind.publishes_levels() {
            crate::group_panes::scene(
                &snapshot,
                meas,
                &status,
                &self.view,
                &self.cache,
                size,
                self.spectrograph.as_ref().map(|(_, h)| h),
            )
        } else if matches!(meas.config.kind, ac2_proto::model::MeasKind::Spl { .. }) {
            crate::modes::spl(
                &snapshot,
                meas,
                &status,
                &self.view,
                size,
                &mut self.spl_hold,
            )
        } else {
            current.and_then(|f| {
                crate::panes::live(
                    &f.frame,
                    f.frame
                        .stamp
                        .grid_id
                        .and_then(|id| snapshot.grids.get(&id).map(AsRef::as_ref)),
                    &meas.config.name,
                    freshness,
                    &status,
                    &self.view,
                    &self.cache,
                    size,
                    &mut self.spl_hold,
                )
            })
        };
        if let Some(scene) = scene {
            plot::paint(ui, plot::PlotSlot(0), rect, Arc::new(scene));
        } else {
            Self::message(
                ui,
                rect,
                if topic.is_none() {
                    if snapshot
                        .mirror
                        .as_ref()
                        .and_then(|m| m.state.as_ref())
                        .is_some_and(|s| {
                            s.traces
                                .iter()
                                .any(|t| t.kind == ac2_proto::model::TraceKind::Sweep)
                        })
                    {
                        "Loading stored sweep result"
                    } else {
                        "No sweep result stored on this rig"
                    }
                } else {
                    "Waiting for measurement data / grid"
                },
            );
        }
        self.step_measurement(&measurements);
    }
}
