use std::process::Command;
use std::sync::mpsc::Sender;

use eframe::egui;

use crate::models::{CommitInfo, HistoryMsg};

/// Number of commits fetched for the history view.
pub(crate) const HISTORY_LIMIT: usize = 100;

/// Field separator used in the git pretty-format (unit separator, unlikely in commit text).
const FIELD_SEP: char = '\u{1f}';

/// Loads the recent commit history of `repo_path` on a background thread and
/// sends the result back through `tx`.
pub(crate) fn load_history(repo_path: String, tx: Sender<HistoryMsg>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let result = run_git_log(&repo_path);
        let _ = tx.send(HistoryMsg::Done(result));
        ctx.request_repaint();
    });
}

fn run_git_log(repo_path: &str) -> Result<Vec<CommitInfo>, String> {
    let format = format!(
        "%H{sep}%h{sep}%an{sep}%ad{sep}%s",
        sep = FIELD_SEP
    );

    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("log")
        .arg(format!("-n{HISTORY_LIMIT}"))
        .arg(format!("--pretty=format:{format}"))
        .arg("--date=format-local:%Y-%m-%d %H:%M")
        .output()
        .map_err(|e| {
            format!(
                "Impossibile eseguire 'git': {e}\n\
                 Verifica che git sia installato e presente nel PATH."
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim();
        return Err(if msg.is_empty() {
            format!(
                "git ha restituito codice {}.",
                output.status.code().unwrap_or(-1)
            )
        } else {
            msg.to_string()
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let commits: Vec<CommitInfo> = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(parse_line)
        .collect();

    Ok(commits)
}

fn parse_line(line: &str) -> Option<CommitInfo> {
    let mut parts = line.split(FIELD_SEP);
    let full_hash = parts.next()?.to_string();
    let short_hash = parts.next()?.to_string();
    let author = parts.next()?.to_string();
    let date = parts.next()?.to_string();
    let subject = parts.next().unwrap_or("").to_string();
    Some(CommitInfo {
        short_hash,
        full_hash,
        author,
        date,
        subject,
    })
}
