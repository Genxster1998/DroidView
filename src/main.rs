#![cfg_attr(windows, windows_subsystem = "windows")]

/*
 * DroidView - A simple, pluggable, graphical user interface for scrcpy
 * Copyright (C) 2024 Genxster1998 <ck.2229.ck@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use clap::Parser;
use droid_view::app::DroidViewApp;
use droid_view::config::AppConfig;
use droid_view::logging::init_logging;
use eframe::{egui, NativeOptions};
use egui::IconData;
use std::sync::Arc;
use tokio::sync::Mutex;
use droid_view::app::ICON_PNG;
use egui_phosphor;

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
    let icon = if let Ok(img) = image::load_from_memory(ICON_PNG) {
        let img = img.to_rgba8();
        let (width, height) = img.dimensions();
        let rgba = img.into_raw();
        Some(Arc::new(IconData {
            rgba,
            width,
            height,
        }))
    } else {
        eprintln!("Warning: Could not load embedded app icon");
        None
    };
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    // --- END ICON LOADING ---

    let native_options = NativeOptions {
        viewport,
        vsync: true,  // Enable vsync for smoother rendering
        multisampling: 0,  // Disable multisampling for better performance
        depth_buffer: 0,   // Disable depth buffer since we don't need 3D
        stencil_buffer: 0, // Disable stencil buffer
        ..Default::default()
    };

    let debug_disable_scrcpy = args.debug_disable_scrcpy;

    // Create and run the application
    eframe::run_native(
        "DroidView",
        native_options,
        Box::new(move |cc| {
            // Register Phosphor icons font
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Fill);
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(DroidViewApp::new(cc, config, debug_disable_scrcpy)))
        }),
    )
}
