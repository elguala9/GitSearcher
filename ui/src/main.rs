#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod git_log;
mod models;
mod scanner;
mod utils;

use app::App;
use config::load_config;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let (config, config_path, config_error) = load_config();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 600.0])
            .with_min_inner_size([640.0, 400.0])
            .with_title("GitSearcher UI"),
        ..Default::default()
    };

    eframe::run_native(
        "GitSearcher UI",
        options,
        Box::new(move |_cc| Ok(Box::new(App::new(config, config_path, config_error)))),
    )
}
