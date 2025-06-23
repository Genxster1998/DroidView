use clap::Parser;
use droid_view::app::DroidViewApp;
use droid_view::config::AppConfig;
use droid_view::logging::init_logging;
use eframe::{egui, NativeOptions};
use egui::IconData;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Set the default theme
    #[arg(short, long, default_value = "default")]
    theme: String,

    /// Show window manager border frame
    #[arg(short = 'W', long, default_value = "false")]
    hide_wm_frame: bool,

    /// Forces the panels to be always on top
    #[arg(short = 'A', long, default_value = "true")]
    always_on_top: bool,

    /// Do not launch scrcpy even when 'Start Scrcpy' is pressed (debug mode)
    #[arg(long)]
    debug_disable_scrcpy: bool,

    /// Reset configuration files
    #[arg(short, long)]
    reset_config: bool,
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let args = Args::parse();

    // Initialize logging
    init_logging();

    // Load or create configuration
    let config = if args.reset_config {
        AppConfig::default()
    } else {
        AppConfig::load().unwrap_or_default()
    };

    // Create shared configuration
    let config = Arc::new(Mutex::new(config));

    // Set up native options
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 600.0])
        .with_min_inner_size([500.0, 400.0])
        .with_decorations(!args.hide_wm_frame);

    if args.always_on_top {
        viewport = viewport.with_always_on_top();
    }

    // --- ICON LOADING ---
    let icon_path = Path::new("assets/icon.png");
    let icon = if let Ok(img) = image::open(icon_path) {
        let img = img.to_rgba8();
        let (width, height) = img.dimensions();
        let rgba = img.into_raw();
        Some(Arc::new(IconData {
            rgba,
            width,
            height,
        }))
    } else {
        eprintln!(
            "Warning: Could not load app icon at {}",
            icon_path.display()
        );
        None
    };
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    // --- END ICON LOADING ---

    let native_options = NativeOptions {
        viewport,
        ..Default::default()
    };

    let debug_disable_scrcpy = args.debug_disable_scrcpy;

    // Create and run the application
    eframe::run_native(
        "DroidView",
        native_options,
        Box::new(move |cc| Box::new(DroidViewApp::new(cc, config, debug_disable_scrcpy))),
    )
}
