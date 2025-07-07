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

pub mod app;
pub mod bridge;
pub mod config;
pub mod device;
pub mod logging;
pub mod ui;
pub mod utils;

pub use app::DroidViewApp;
pub use config::AppConfig;
