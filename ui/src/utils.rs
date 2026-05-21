use chrono::{DateTime, Utc};
use eframe::egui;

pub(crate) fn sort_header(label: &str, active: bool, asc: bool) -> egui::Button<'_> {
    let arrow = if !active { "" } else if asc { " ▲" } else { " ▼" };
    let txt = format!("{label}{arrow}");
    egui::Button::new(egui::RichText::new(txt).strong()).frame(false)
}

pub(crate) fn format_time(t: Option<DateTime<Utc>>) -> String {
    match t {
        Some(dt) => dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string(),
        None => "-".into(),
    }
}

pub(crate) fn dirs_home() -> String {
    if let Some(h) = std::env::var_os("USERPROFILE") {
        return h.to_string_lossy().into_owned();
    }
    if let Some(h) = std::env::var_os("HOME") {
        return h.to_string_lossy().into_owned();
    }
    String::from(".")
}
