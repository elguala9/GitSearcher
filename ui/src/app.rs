use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::config::Config;
use crate::git_log::load_history;
use crate::models::{CommitInfo, HistoryMsg, RepoInfo, ScanMsg, SortBy};
use crate::scanner::{resolve_executable, run_scan};
use crate::utils::{dirs_home, format_time, sort_header};

pub(crate) struct App {
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

    history_open: bool,
    history_repo: Option<String>,
    history_branch: Option<String>,
    history_commits: Vec<CommitInfo>,
    history_loading: bool,
    history_error: Option<String>,
    history_rx: Option<Receiver<HistoryMsg>>,
}

impl App {
    pub(crate) fn new(
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
            history_open: false,
            history_repo: None,
            history_branch: None,
            history_commits: Vec::new(),
            history_loading: false,
            history_error: None,
            history_rx: None,
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

    fn open_history(&mut self, repo: &RepoInfo, ctx: &egui::Context) {
        self.history_open = true;
        self.history_repo = Some(repo.path.clone());
        self.history_branch = repo.branch.clone();
        self.history_commits.clear();
        self.history_error = None;
        self.history_loading = true;

        let (tx, rx) = channel::<HistoryMsg>();
        self.history_rx = Some(rx);
        load_history(repo.path.clone(), tx, ctx.clone());
    }

    fn drain_history(&mut self) {
        let Some(rx) = self.history_rx.take() else { return };
        let mut keep = true;
        loop {
            match rx.try_recv() {
                Ok(HistoryMsg::Done(result)) => {
                    self.history_loading = false;
                    keep = false;
                    match result {
                        Ok(commits) => self.history_commits = commits,
                        Err(e) => self.history_error = Some(e),
                    }
                }
                Err(_) => break,
            }
        }
        if keep {
            self.history_rx = Some(rx);
        }
    }

    fn show_history_window(&mut self, ctx: &egui::Context) {
        if !self.history_open {
            return;
        }
        if self.history_loading {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        let mut open = self.history_open;
        let title = match &self.history_repo {
            Some(p) => format!(
                "Storia — {}",
                Path::new(p)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| p.clone())
            ),
            None => "Storia".to_string(),
        };

        egui::Window::new(title)
            .open(&mut open)
            .resizable(true)
            .default_width(720.0)
            .default_height(440.0)
            .collapsible(true)
            .show(ctx, |ui| {
                if let Some(p) = &self.history_repo {
                    ui.horizontal(|ui| {
                        ui.weak(p);
                        if let Some(b) = &self.history_branch {
                            ui.separator();
                            ui.label(format!("branch: {b}"));
                        }
                    });
                }
                ui.separator();

                if self.history_loading {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Caricamento commit…");
                    });
                    return;
                }

                if let Some(err) = &self.history_error {
                    ui.colored_label(egui::Color32::LIGHT_RED, err);
                    return;
                }

                if self.history_commits.is_empty() {
                    ui.weak("Nessun commit trovato.");
                    return;
                }

                ui.label(format!("Ultimi {} commit:", self.history_commits.len()));
                ui.add_space(4.0);

                let mut copy_hash: Option<String> = None;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for c in &self.history_commits {
                            ui.horizontal(|ui| {
                                let hash = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&c.short_hash)
                                            .monospace()
                                            .color(egui::Color32::from_rgb(0xE5, 0xA5, 0x4B)),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if hash.clicked() {
                                    copy_hash = Some(c.full_hash.clone());
                                }
                                hash.on_hover_text("Clic per copiare l'hash completo");
                                ui.weak(&c.date);
                                ui.separator();
                                ui.label(&c.subject);
                            });
                            ui.horizontal(|ui| {
                                ui.add_space(4.0);
                                ui.small(format!("— {}", c.author));
                            });
                            ui.add_space(2.0);
                            ui.separator();
                        }
                    });

                if let Some(h) = copy_hash {
                    ui.output_mut(|o| o.copied_text = h);
                }
            });

        self.history_open = open;
        if !self.history_open {
            self.history_rx = None;
            self.history_loading = false;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();
        self.drain_history();
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

        egui::TopBottomPanel::bottom("bottom")
            .resizable(true)
            .default_height(140.0)
            .show(ctx, |ui| {
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
            let mut history_request: Option<usize> = None;
            let mut sort_request: Option<SortBy> = None;

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(170.0).at_least(120.0))
                .column(Column::initial(130.0).at_least(80.0))
                .column(Column::remainder().at_least(200.0))
                .column(Column::initial(110.0).at_least(80.0))
                .column(Column::initial(150.0).at_least(120.0))
                .header(22.0, |mut header| {
                    header.col(|ui| {
                        if ui
                            .add(sort_header(
                                "Ultima attività",
                                self.sort_by == SortBy::LastActivity,
                                self.sort_asc,
                            ))
                            .clicked()
                        {
                            sort_request = Some(SortBy::LastActivity);
                        }
                    });
                    header.col(|ui| {
                        if ui
                            .add(sort_header(
                                "Branch",
                                self.sort_by == SortBy::Branch,
                                self.sort_asc,
                            ))
                            .clicked()
                        {
                            sort_request = Some(SortBy::Branch);
                        }
                    });
                    header.col(|ui| {
                        if ui
                            .add(sort_header(
                                "Percorso",
                                self.sort_by == SortBy::Path,
                                self.sort_asc,
                            ))
                            .clicked()
                        {
                            sort_request = Some(SortBy::Path);
                        }
                    });
                    header.col(|ui| {
                        ui.strong("Fonte");
                    });
                    header.col(|ui| {
                        ui.strong("Azioni");
                    });
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
                                    history_request = Some(idx);
                                }
                                resp.on_hover_text("Doppio clic per vedere la storia dei commit");
                            });
                            row.col(|ui| {
                                ui.label(repo.source.as_deref().unwrap_or("-"));
                            });
                            row.col(|ui| {
                                if ui.small_button("Storia").clicked() {
                                    history_request = Some(idx);
                                }
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
            if let Some(idx) = history_request {
                let repo = self.repos[idx].clone();
                self.open_history(&repo, ctx);
            }

            if self.repos.is_empty() && !self.scanning {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.weak("Nessun repository ancora. Seleziona un percorso e premi Scansiona.");
                });
            }
        });

        self.show_history_window(ctx);
    }
}
