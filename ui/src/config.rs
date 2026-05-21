use std::path::PathBuf;

use serde::Deserialize;

pub(crate) const CONFIG_FILE_NAME: &str = "gitsearcher-ui.toml";

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    pub(crate) executable: String,
    pub(crate) extra_args: Vec<String>,
    pub(crate) default_path: String,
    pub(crate) include_hidden: bool,
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

pub(crate) fn load_config() -> (Config, Option<PathBuf>, Option<String>) {
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
