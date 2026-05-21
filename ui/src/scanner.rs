use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use eframe::egui;

use crate::models::{RepoInfo, ScanMsg};

pub(crate) fn resolve_executable(value: &str, config_path: Option<&Path>) -> PathBuf {
    let p = Path::new(value);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    if let Some(cfg) = config_path {
        if let Some(dir) = cfg.parent() {
            let candidate = dir.join(p);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(p);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    PathBuf::from(value)
}

pub(crate) fn run_scan(
    exe: &Path,
    path: &str,
    include_hidden: bool,
    extra_args: &[String],
    tx: Sender<ScanMsg>,
    ctx: egui::Context,
) {
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
