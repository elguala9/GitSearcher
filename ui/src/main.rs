#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use chrono::{DateTime, Utc};
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use serde::Deserialize;

const CONFIG_FILE_NAME: &str = "gitsearcher-ui.toml";

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
        Box::new(move |_cc| {
            Ok(Box::new(App::new(config, config_path, config_error)))
        }),
    )
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct Config {
    executable: String,
    extra_args: Vec<String>,
    default_path: String,
    include_hidden: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            executable: "GitSearcher.exe".into(),
            extra_args: vec![],
            default_path: String::new(),
            include_hidden: false,
        }
    }
}

fn load_config() -> (Config, Option<PathBuf>, Option<String>) {
    let candidates = config_search_paths();
    for p in &candidates {
        if p.is_file() {
            match std::fs::read_to_string(p) {
                Ok(text) => match toml::from_str::<Config>(&text) {
                    Ok(cfg) => return (cfg, Some(p.clone()), None),
                    Err(e) => {
                        return (
                            Config::default(),
                            Some(p.clone()),
                            Some(format!("Errore parsing {}: {}", p.display(), e)),
                        )
                    }
                },
                Err(e) => {
                    return (
                        Config::default(),
                        Some(p.clone()),
                        Some(format!("Errore lettura {}: {}", p.display(), e)),
                    )
                }
            }
        }
    }
    (Config::default(), None, None)
}

fn config_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join(CONFIG_FILE_NAME));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(CONFIG_FILE_NAME));
    }
    paths
}

#[derive(Debug, Clone, Deserialize)]
struct RepoInfo {
    path: String,
    branch: Option<String>,
    #[serde(rename = "lastActivityUtc")]
    last_activity_utc: Option<DateTime<Utc>>,
    source: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortBy {
    LastActivity,
    Path,
    Branch,
}

enum ScanMsg {
    Log(String),
    Done(Result<Vec<RepoInfo>, String>),
}

struct App {
    config: Config,
    config_path: Option<PathBuf>,
    config_error: Option<String>,

    path_input: String,
    include_hidden: bool,
    filter: String,
    sort_by: SortBy,
    sort_asc: bool,

    repos: Vec<RepoInfo>,
    logs: Vec<String>,
    scanning: bool,
    last_error: Option<String>,
    rx: Option<Receiver<ScanMsg>>,
}

impl App {
    fn new(
        config: Config,
        config_path: Option<PathBuf>,
        config_error: Option<String>,
    ) -> Self {
        let initial_path = if !config.default_path.is_empty() {
            config.default_path.clone()
        } else {
            dirs_home()
        };
        let include_hidden = config.include_hidden;
        Self {
            config,
            config_path,
            config_error,
            path_input: initial_path,
            include_hidden,
            filter: String::new(),
            sort_by: SortBy::LastActivity,
            sort_asc: false,
            repos: Vec::new(),
            logs: Vec::new(),
            scanning: false,
            last_error: None,
            rx: None,
        }
    }

    fn start_scan(&mut self, ctx: &egui::Context) {
        if self.scanning {
            return;
        }
        let path = self.path_input.trim().to_string();
        if path.is_empty() {
            self.last_error = Some("Inserisci un percorso da scansionare.".into());
            return;
        }
        if !Path::new(&path).is_dir() {
            self.last_error = Some(format!("Il percorso non esiste: {path}"));
            return;
        }

        self.scanning = true;
        self.logs.clear();
        self.repos.clear();
        self.last_error = None;

        let (tx, rx) = channel::<ScanMsg>();
        self.rx = Some(rx);

        let exe_cmd = resolve_executable(&self.config.executable, self.config_path.as_deref());
        let extra_args = self.config.extra_args.clone();
        let include_hidden = self.include_hidden;
        let ctx_repaint = ctx.clone();

        thread::spawn(move || {
            run_scan(&exe_cmd, &path, include_hidden, &extra_args, tx, ctx_repaint);
        });
    }

    fn drain_messages(&mut self) {
        let Some(rx) = self.rx.take() else { return };
        let mut keep = true;
        loop {
            match rx.try_recv() {
                Ok(ScanMsg::Log(line)) => {
                    if self.logs.len() > 500 {
                        self.logs.remove(0);
                    }
                    self.logs.push(line);
                }
                Ok(ScanMsg::Done(result)) => {
                    self.scanning = false;
                    keep = false;
                    match result {
                        Ok(mut list) => {
                            self.repos.append(&mut list);
                            self.apply_sort();
                        }
                        Err(e) => self.last_error = Some(e),
                    }
                }
                Err(_) => break,
            }
        }
        if keep {
            self.rx = Some(rx);
        }
    }

    fn apply_sort(&mut self) {
        let asc = self.sort_asc;
        match self.sort_by {
            SortBy::LastActivity => {
                self.repos.sort_by(|a, b| {
                    let ord = a.last_activity_utc.cmp(&b.last_activity_utc);
                    if asc { ord } else { ord.reverse() }
                });
            }
            SortBy::Path => {
                self.repos.sort_by(|a, b| {
                    let ord = a.path.to_lowercase().cmp(&b.path.to_lowercase());
                    if asc { ord } else { ord.reverse() }
                });
            }
            SortBy::Branch => {
                self.repos.sort_by(|a, b| {
                    let ord = a.branch.as_deref().unwrap_or("")
                        .cmp(b.branch.as_deref().unwrap_or(""));
                    if asc { ord } else { ord.reverse() }
                });
            }
        }
    }

    fn toggle_sort(&mut self, col: SortBy) {
        if self.sort_by == col {
            self.sort_asc = !self.sort_asc;
        } else {
            self.sort_by = col;
            self.sort_asc = matches!(col, SortBy::Path | SortBy::Branch);
        }
        self.apply_sort();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();
        if self.scanning {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Percorso:");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.path_input)
                        .desired_width(ui.available_width() - 220.0),
                );
                if ui.button("Sfoglia…").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.path_input = dir.to_string_lossy().into_owned();
                    }
                }
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let scan_btn = ui.add_enabled(
                    !self.scanning,
                    egui::Button::new(if self.scanning { "Scansione…" } else { "Scansiona" }),
                );
                if scan_btn.clicked() || enter {
                    self.start_scan(ctx);
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.include_hidden, "Includi cartelle nascoste / node_modules");
                ui.separator();
                ui.label("Filtro:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .hint_text("filtra per percorso o branch")
                        .desired_width(260.0),
                );
                if !self.repos.is_empty() {
                    ui.separator();
                    ui.label(format!("{} repo", self.repos.len()));
                }
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("bottom").resizable(true).default_height(140.0).show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.strong("Log");
                ui.separator();
                if let Some(p) = &self.config_path {
                    ui.label(format!("Config: {}", p.display()));
                } else {
                    ui.label("Config: (default — nessun file trovato)");
                }
                if ui.small_button("Apri config").clicked() {
                    if let Some(p) = &self.config_path {
                        let _ = open::that(p);
                    } else if let Ok(exe) = std::env::current_exe() {
                        if let Some(dir) = exe.parent() {
                            let _ = open::that(dir);
                        }
                    }
                }
            });
            if let Some(err) = &self.config_error {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            }
            if let Some(err) = &self.last_error {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            }
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                for line in &self.logs {
                    ui.monospace(line);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let filter = self.filter.trim().to_lowercase();
            let filtered: Vec<usize> = self
                .repos
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    if filter.is_empty() {
                        return true;
                    }
                    r.path.to_lowercase().contains(&filter)
                        || r.branch.as_deref().unwrap_or("").to_lowercase().contains(&filter)
                })
                .map(|(i, _)| i)
                .collect();

            let mut clicked_path: Option<String> = None;
            let mut sort_request: Option<SortBy> = None;

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(170.0).at_least(120.0))
                .column(Column::initial(130.0).at_least(80.0))
                .column(Column::remainder().at_least(200.0))
                .column(Column::initial(110.0).at_least(80.0))
                .column(Column::initial(80.0).at_least(60.0))
                .header(22.0, |mut header| {
                    header.col(|ui| {
                        if ui
                            .add(sort_header("Ultima attività", self.sort_by == SortBy::LastActivity, self.sort_asc))
                            .clicked()
                        {
                            sort_request = Some(SortBy::LastActivity);
                        }
                    });
                    header.col(|ui| {
                        if ui
                            .add(sort_header("Branch", self.sort_by == SortBy::Branch, self.sort_asc))
                            .clicked()
                        {
                            sort_request = Some(SortBy::Branch);
                        }
                    });
                    header.col(|ui| {
                        if ui
                            .add(sort_header("Percorso", self.sort_by == SortBy::Path, self.sort_asc))
                            .clicked()
                        {
                            sort_request = Some(SortBy::Path);
                        }
                    });
                    header.col(|ui| { ui.strong("Fonte"); });
                    header.col(|ui| { ui.strong("Azioni"); });
                })
                .body(|mut body| {
                    for idx in filtered {
                        let repo = &self.repos[idx];
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(format_time(repo.last_activity_utc));
                            });
                            row.col(|ui| {
                                ui.label(repo.branch.as_deref().unwrap_or("-"));
                            });
                            row.col(|ui| {
                                let resp = ui.add(
                                    egui::Label::new(&repo.path)
                                        .sense(egui::Sense::click())
                                        .truncate(),
                                );
                                if resp.double_clicked() {
                                    clicked_path = Some(repo.path.clone());
                                }
                                resp.on_hover_text("Doppio clic per aprire in Esplora risorse");
                            });
                            row.col(|ui| {
                                ui.label(repo.source.as_deref().unwrap_or("-"));
                            });
                            row.col(|ui| {
                                if ui.small_button("Apri").clicked() {
                                    clicked_path = Some(repo.path.clone());
                                }
                            });
                        });
                    }
                });

            if let Some(col) = sort_request {
                self.toggle_sort(col);
            }
            if let Some(p) = clicked_path {
                let _ = open::that(&p);
            }

            if self.repos.is_empty() && !self.scanning {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.weak("Nessun repository ancora. Seleziona un percorso e premi Scansiona.");
                });
            }
        });
    }
}

fn sort_header(label: &str, active: bool, asc: bool) -> egui::Button<'_> {
    let arrow = if !active {
        ""
    } else if asc {
        " ▲"
    } else {
        " ▼"
    };
    let txt = format!("{label}{arrow}");
    egui::Button::new(egui::RichText::new(txt).strong()).frame(false)
}

fn format_time(t: Option<DateTime<Utc>>) -> String {
    match t {
        Some(dt) => dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string(),
        None => "-".into(),
    }
}

fn dirs_home() -> String {
    if let Some(h) = std::env::var_os("USERPROFILE") {
        return h.to_string_lossy().into_owned();
    }
    if let Some(h) = std::env::var_os("HOME") {
        return h.to_string_lossy().into_owned();
    }
    String::from(".")
}

fn resolve_executable(value: &str, config_path: Option<&Path>) -> PathBuf {
    let p = Path::new(value);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    // try relative to the config file's directory
    if let Some(cfg) = config_path {
        if let Some(dir) = cfg.parent() {
            let candidate = dir.join(p);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    // try relative to the UI executable's directory
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(p);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    // fall back to the bare value so the OS resolves it via PATH
    PathBuf::from(value)
}

fn run_scan(
    exe: &Path,
    path: &str,
    include_hidden: bool,
    extra_args: &[String],
    tx: Sender<ScanMsg>,
    ctx: egui::Context,
) {
    // place the JSON in the system temp dir
    let mut out_path = std::env::temp_dir();
    out_path.push(format!("gitsearcher-ui-{}.json", std::process::id()));

    let mut cmd = Command::new(exe);
    cmd.arg(path).arg("-o").arg(&out_path);
    if include_hidden {
        cmd.arg("--include-hidden");
    }
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let _ = tx.send(ScanMsg::Log(format!(
        "$ {} {}",
        exe.display(),
        format_args_preview(path, include_hidden, extra_args, &out_path)
    )));
    ctx.request_repaint();

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let msg = format!(
                "Impossibile avviare l'eseguibile '{}': {}\n\
                 Verifica che il file esista o che sia nel PATH, e controlla il campo 'executable' nel file di config.",
                exe.display(),
                e
            );
            let _ = tx.send(ScanMsg::Done(Err(msg)));
            ctx.request_repaint();
            return;
        }
    };

    // stream stderr (progress) to the UI
    if let Some(stderr) = child.stderr.take() {
        let tx2 = tx.clone();
        let ctx2 = ctx.clone();
        thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx2.send(ScanMsg::Log(line));
                ctx2.request_repaint();
            }
        });
    }

    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => {
            let _ = tx.send(ScanMsg::Done(Err(format!("Errore esecuzione: {e}"))));
            ctx.request_repaint();
            return;
        }
    };

    if !status.success() {
        let _ = tx.send(ScanMsg::Done(Err(format!(
            "L'eseguibile ha restituito codice {}.",
            status.code().unwrap_or(-1)
        ))));
        ctx.request_repaint();
        return;
    }

    let json = match std::fs::read_to_string(&out_path) {
        Ok(s) => s,
        Err(e) => {
            let _ = tx.send(ScanMsg::Done(Err(format!(
                "Impossibile leggere l'output JSON ({}): {}",
                out_path.display(),
                e
            ))));
            ctx.request_repaint();
            return;
        }
    };

    let parsed: Result<Vec<RepoInfo>, _> = serde_json::from_str(&json);
    let _ = std::fs::remove_file(&out_path);

    match parsed {
        Ok(list) => {
            let _ = tx.send(ScanMsg::Done(Ok(list)));
        }
        Err(e) => {
            let _ = tx.send(ScanMsg::Done(Err(format!("JSON non valido: {e}"))));
        }
    }
    ctx.request_repaint();
}

fn format_args_preview(path: &str, include_hidden: bool, extra: &[String], out: &Path) -> String {
    let mut s = format!("\"{path}\" -o \"{}\"", out.display());
    if include_hidden {
        s.push_str(" --include-hidden");
    }
    for a in extra {
        s.push(' ');
        s.push_str(a);
    }
    s
}
