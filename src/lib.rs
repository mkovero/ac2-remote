mod app;
mod discovery;
mod gestures;
mod group_panes;
mod groups;
pub mod link;
mod modes;
#[cfg(target_os = "android")]
mod multicast;
mod pairing;
mod panes;
// Reuse the desktop GPU adapter directly; extract it into a shared crate upstream later.
#[path = "../../ac2/crates/ac2-ui/src/plot.rs"]
mod plot;

pub fn run(
    options: eframe::NativeOptions,
    data_dir: std::path::PathBuf,
    demo: bool,
) -> eframe::Result {
    eframe::run_native(
        "ac2 Remote",
        options,
        Box::new(move |cc| {
            let rs = cc
                .wgpu_render_state
                .as_ref()
                .ok_or("wgpu renderer required")?;
            plot::install(rs);
            Ok(Box::new(app::RemoteApp::new(data_dir, demo)))
        }),
    )
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    app.set_window_flags(
        android_activity::WindowManagerFlags::FULLSCREEN,
        android_activity::WindowManagerFlags::FORCE_NOT_FULLSCREEN,
    );
    if let Err(error) = multicast::landscape() {
        eprintln!("ac2 Remote: {error}");
    }
    let Some(data_dir) = app.internal_data_path() else {
        eprintln!("ac2 Remote: Android app-private storage unavailable");
        return;
    };
    let options = eframe::NativeOptions {
        android_app: Some(app),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    if let Err(error) = run(options, data_dir, false) {
        eprintln!("ac2 Remote: {error}");
    }
}
