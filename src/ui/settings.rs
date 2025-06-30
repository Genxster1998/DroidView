use crate::config::AppConfig;
use egui::{Ui, Window};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SettingsWindow {
    visible: bool,
    config: Arc<Mutex<AppConfig>>,
    just_saved: bool,
}

enum SettingsResult {
    Save,
    Close,
    Reset,
    Nothing,
}

impl SettingsWindow {
    pub fn new(config: Arc<Mutex<AppConfig>>) -> Self {
        Self {
            visible: false,
            config,
            just_saved: false,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let mut open = self.visible;
        let config_lock = self.config.try_lock();
        if let Ok(mut config) = config_lock {
            let response = Window::new("Settings")
                .open(&mut open)
                .resizable(true)
                .default_size([400.0, 500.0])
                .show(ctx, |ui| show_settings_content(ui, &mut config));

            if let Some(inner) = response.and_then(|r| r.inner) {
                match inner {
                    SettingsResult::Save => {
                        config.save().ok();
                        self.visible = false;
                        self.just_saved = true;
                    }
                    SettingsResult::Close => {
                        self.visible = false;
                    }
                    SettingsResult::Reset => {
                        // Already updated in show_settings_content
                    }
                    SettingsResult::Nothing => {}
                }
            }
        }

        if !open {
            self.visible = false;
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn take_just_saved(&mut self) -> bool {
        let was = self.just_saved;
        self.just_saved = false;
        was
    }
}

fn show_settings_content(ui: &mut Ui, config: &mut AppConfig) -> SettingsResult {
    let mut result = SettingsResult::Nothing;

    ui.heading("Application Settings");

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Paths section
        ui.group(|ui| {
            ui.heading("Paths");

            ui.label("ADB Path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(config.adb_path.get_or_insert_with(String::new));
                if ui.button("Browse").clicked() {
                    // TODO: Implement file picker
                }
            });

            ui.label("Scrcpy Path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(config.scrcpy_path.get_or_insert_with(String::new));
                if ui.button("Browse").clicked() {
                    // TODO: Implement file picker
                }
            });
        });

        // Video settings
        ui.group(|ui| {
            ui.heading("Video Settings");

            // Bitrate selection with K/M units
            let mut bitrate_value: u32 = {
                // Parse the numeric part from config.bitrate
                let s = config.bitrate.trim().to_uppercase();
                if s.ends_with('M') {
                    s.trim_end_matches('M').parse::<u32>().unwrap_or(8) * 1000
                } else if s.ends_with('K') {
                    s.trim_end_matches('K').parse::<u32>().unwrap_or(8000)
                } else {
                    s.parse::<u32>().unwrap_or(8000)
                }
            };
            let mut bitrate_unit = if config.bitrate.trim().to_uppercase().ends_with('M') {
                "Mbps"
            } else {
                "Kbps"
            };

            ui.horizontal(|ui| {
                ui.label("Bitrate:");
                ui.add(egui::Slider::new(&mut bitrate_value, 100..=20000).text("Value"));
                egui::ComboBox::from_id_salt("bitrate_unit_combo")
                    .selected_text(bitrate_unit)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut bitrate_unit, "Kbps", "Kbps");
                        ui.selectable_value(&mut bitrate_unit, "Mbps", "Mbps");
                    });
            });
            let bitrate_str = if bitrate_unit == "Mbps" {
                format!("{}M", (bitrate_value as f32 / 1000.0).round() as u32)
            } else {
                format!("{}K", bitrate_value)
            };
            config.bitrate = bitrate_str;
            ui.label(format!("Current: {}", config.bitrate));

            ui.label("Orientation:");
            let orientations = [
                (None, "Default"),
                (Some("0"), "0°"),
                (Some("90"), "90°"),
                (Some("180"), "180°"),
                (Some("270"), "270°"),
                (Some("flip0"), "Flip 0°"),
                (Some("flip90"), "Flip 90°"),
                (Some("flip180"), "Flip 180°"),
                (Some("flip270"), "Flip 270°"),
            ];
            egui::ComboBox::from_id_salt("orientation_combo")
                .selected_text(
                    orientations
                        .iter()
                        .find(|(val, _)| val.as_ref().map(|v| v.to_string()) == config.orientation)
                        .map(|(_, label)| *label)
                        .unwrap_or("Default"),
                )
                .show_ui(ui, |ui| {
                    for (val, label) in orientations.iter() {
                        let selected = config
                            .orientation
                            .as_ref()
                            .map(|v| v == &val.unwrap_or("").to_string())
                            .unwrap_or(val.is_none());
                        if ui.selectable_label(selected, *label).clicked() {
                            config.orientation = val.map(|v| v.to_string());
                        }
                    }
                });

            ui.checkbox(&mut config.show_touches, "Show touches");
            ui.checkbox(&mut config.turn_screen_off, "Turn screen off");
            ui.checkbox(&mut config.fullscreen, "Fullscreen");

            ui.label("Max dimension:");
            ui.horizontal(|ui| {
                let mut custom_dim = config.dimension.is_some();
                if ui.checkbox(&mut custom_dim, "Custom").changed() {
                    if !custom_dim {
                        config.dimension = None;
                    } else {
                        config.dimension = Some(800); // default value if enabling
                    }
                }
                if let Some(ref mut dim) = config.dimension {
                    ui.add(
                        egui::DragValue::new(dim)
                            .suffix("px")
                            .range(100..=10000),
                    );
                }
            });

            ui.checkbox(&mut config.force_adb_forward, "Force ADB Forward (--force-adb-forward)");
        });

        // Panels
        ui.group(|ui| {
            ui.heading("Panels");
            ui.checkbox(&mut config.panels.swipe, "Swipe Panel");
            ui.checkbox(&mut config.panels.toolkit, "Toolkit Panel");
            ui.checkbox(&mut config.panels.bottom, "Bottom Panel");
        });

        // Extra arguments
        ui.group(|ui| {
            ui.heading("Extra Arguments");
            ui.label("Additional scrcpy arguments:");
            ui.text_edit_multiline(&mut config.extra_args);
        });

        // Theme
        ui.group(|ui| {
            ui.heading("Theme");
            ui.horizontal(|ui| {
                ui.radio_value(&mut config.theme, "default".to_string(), "Default");
                ui.radio_value(&mut config.theme, "dark".to_string(), "Dark");
                ui.radio_value(&mut config.theme, "light".to_string(), "Light");
            });
        });
    });

    // Buttons
    ui.horizontal(|ui| {
        if ui.button("💾 Save").clicked() {
            result = SettingsResult::Save;
        }

        if ui.button("❌ Cancel").clicked() {
            result = SettingsResult::Close;
        }

        if ui.button("🔄 Reset to Defaults").clicked() {
            *config = AppConfig::default();
            result = SettingsResult::Reset;
        }
    });

    result
}
