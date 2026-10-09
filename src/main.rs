fn main() -> eframe::Result {
    let data_dir = std::env::var_os("AC2_REMOTE_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(".local-data"));
    ac2_remote::run(
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: eframe::egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
            ..Default::default()
        },
        data_dir,
        std::env::args().any(|arg| arg == "--demo"),
    )
}
