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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEntry {
    pub name: String,
    pub github_token: Option<String>,
    #[serde(default)]
    pub repos: Vec<RepoEntry>,
    #[serde(default)]
    pub trunk_branches: Vec<String>,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_max_commits")]
    pub max_commits: usize,
    #[serde(default = "default_show_forks")]
    pub show_forks: bool,
    pub theme: Option<String>,
}

fn default_poll_interval() -> u64 {
    60
}
fn default_max_commits() -> usize {
    500
}
fn default_show_forks() -> bool {
    true
}

impl Default for ProfileEntry {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            github_token: None,
            repos: Vec::new(),
            trunk_branches: Vec::new(),
            poll_interval_secs: 60,
            max_commits: 500,
            show_forks: true,
            theme: None,
        }
    }
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
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub active_profile: Option<String>,
    #[serde(default)]
    pub profiles: Vec<ProfileEntry>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("repo_path", &self.repo_path)
            .field(
                "github_token",
                &self.github_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("poll_interval_secs", &self.poll_interval_secs)
            .field("show_forks", &self.show_forks)
            .field("max_commits", &self.max_commits)
            .field("repos", &self.repos)
            .field("trunk_branches", &self.trunk_branches)
            .field("theme", &self.theme)
            .field("active_profile", &self.active_profile)
            .field("profiles_count", &self.profiles.len())
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
            theme: None,
            active_profile: None,
            profiles: Vec::new(),
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

        let mut config: Config = match figment.extract() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("warning: config parse error, using defaults: {e}");
                Config::default()
            }
        };

        config.apply_active_profile();
        config
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

    pub fn save(&self) -> std::io::Result<()> {
        let dir = config_dir().join("arachne");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("config.toml");
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, content)
    }

    pub fn config_file_exists() -> bool {
        config_dir().join("arachne").join("config.toml").exists()
    }

    fn apply_active_profile(&mut self) {
        let active = match &self.active_profile {
            Some(name) => name.clone(),
            None => return,
        };
        let profile = match self.profiles.iter().find(|p| p.name == active) {
            Some(p) => p.clone(),
            None => return,
        };
        if profile.github_token.is_some() {
            self.github_token = profile.github_token;
        }
        if !profile.repos.is_empty() {
            self.repos = profile.repos;
        }
        if !profile.trunk_branches.is_empty() {
            self.trunk_branches = profile.trunk_branches;
        }
        self.poll_interval_secs = profile.poll_interval_secs;
        self.max_commits = profile.max_commits;
        self.show_forks = profile.show_forks;
        if profile.theme.is_some() {
            self.theme = profile.theme;
        }
    }
}

pub fn config_dir() -> PathBuf {
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
