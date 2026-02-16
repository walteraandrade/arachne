use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoEntry {
    pub path: PathBuf,
    pub name: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub repo_path: PathBuf,
    pub github_token: Option<String>,
    pub poll_interval_secs: u64,
    pub show_forks: bool,
    pub max_commits: usize,
    #[serde(default)]
    pub repos: Vec<RepoEntry>,
    #[serde(default = "default_trunk_branches")]
    pub trunk_branches: Vec<String>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("repo_path", &self.repo_path)
            .field("github_token", &self.github_token.as_ref().map(|_| "[REDACTED]"))
            .field("poll_interval_secs", &self.poll_interval_secs)
            .field("show_forks", &self.show_forks)
            .field("max_commits", &self.max_commits)
            .field("repos", &self.repos)
            .field("trunk_branches", &self.trunk_branches)
            .finish()
    }
}

fn default_trunk_branches() -> Vec<String> {
    vec![
        "development".to_string(),
        "staging".to_string(),
        "production".to_string(),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::from("."),
            github_token: None,
            poll_interval_secs: 60,
            show_forks: true,
            max_commits: 500,
            repos: Vec::new(),
            trunk_branches: default_trunk_branches(),
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

        match figment.extract() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("warning: config parse error, using defaults: {e}");
                Config::default()
            }
        }
    }

    pub fn resolved_repos(&self) -> Vec<RepoEntry> {
        if self.repos.is_empty() {
            vec![RepoEntry {
                path: self.repo_path.clone(),
                name: None,
            }]
        } else {
            self.repos.clone()
        }
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
