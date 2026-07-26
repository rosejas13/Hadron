// Special-paths config. Persisted as a minimal `name=value` text file,
// compatible with the Java Properties file the original Quark wrote (so an
// existing Quark config can be copied over if migrating).

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Config {
    pub path: PathBuf,
    pub paths: BTreeMap<String, String>,
}

impl Config {
    pub fn empty(path: PathBuf) -> Self {
        Config { path, paths: BTreeMap::new() }
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let mut cfg = Config::empty(path.to_path_buf());
        if path.is_file() {
            for line in fs::read_to_string(path)?.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some(eq) = trimmed.find('=') {
                    let key = trimmed[..eq].trim().to_string();
                    let value = trimmed[eq + 1..].trim().to_string();
                    if !key.is_empty() {
                        cfg.paths.insert(key, value);
                    }
                }
            }
        }
        Ok(cfg)
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = String::new();
        for (k, v) in &self.paths {
            out.push_str(k);
            out.push('=');
            out.push_str(v);
            out.push('\n');
        }
        let mut f = fs::File::create(&self.path)?;
        f.write_all(out.as_bytes())?;
        Ok(())
    }

    pub fn add(&mut self, name: &str, path: &str) {
        self.paths.insert(name.to_string(), path.to_string());
    }

    pub fn remove(&mut self, names: &[String]) {
        for n in names {
            self.paths.remove(n);
        }
    }

    pub fn entries(&self) -> Vec<(String, String)> {
        self.paths.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

pub fn default_config_path() -> PathBuf {
    if cfg!(windows) {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into()))
            });
        base.join("hadron").join("hadron-config.cfg")
    } else {
        let config_home = std::env::var("XDG_CONFIG_HOME").ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".config")
            });
        config_home.join("hadron").join("hadron-config.cfg")
    }
}