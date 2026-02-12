use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub repo_path: PathBuf,
    pub github_token: Option<String>,
    pub poll_interval_secs: u64,
    pub show_forks: bool,
    pub max_commits: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::from("."),
            github_token: None,
            poll_interval_secs: 60,
            show_forks: true,
            max_commits: 500,
        }
    }
}

impl Config {
    pub fn load(cli_path: Option<PathBuf>) -> Self {
        let config_file = config_dir().join("arachne").join("config.toml");

        let mut figment = Figment::from(Serialized::defaults(Config::default()));

        if config_file.exists() {
            figment = figment.merge(Toml::file(&config_file));
        }

        figment = figment.merge(Env::prefixed("ARACHNE_")).merge(
            Env::raw()
                .only(&["GITHUB_TOKEN"])
                .map(|_| "github_token".into()),
        );

        if let Some(path) = cli_path {
            figment = figment.merge(Serialized::default("repo_path", path));
        }

        figment.extract().unwrap_or_default()
    }
}

fn config_dir() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."))
}
