use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(rename = "serverAddr")]
    pub server_addr: String,
    #[serde(rename = "serverPort")]
    pub server_port: u16,
    pub auth: Auth,
    #[serde(default)]
    pub proxies: Vec<Proxy>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Auth {
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Proxy {
    pub name: String,
    #[serde(rename = "type", default = "default_tcp")]
    pub kind: String,
    #[serde(rename = "localIP", default = "default_local")]
    pub local_ip: String,
    #[serde(rename = "localPort")]
    pub local_port: u16,
    #[serde(rename = "remotePort")]
    pub remote_port: u16,
}

fn default_method() -> String {
    "token".into()
}

fn default_tcp() -> String {
    "tcp".into()
}

fn default_local() -> String {
    "127.0.0.1".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_addr: String::new(),
            server_port: 7000,
            auth: Auth {
                method: default_method(),
                token: String::new(),
            },
            proxies: vec![Proxy {
                name: "mc".into(),
                kind: default_tcp(),
                local_ip: default_local(),
                local_port: 25565,
                remote_port: 25565,
            }],
        }
    }
}

pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        exe_dir()
    }
    #[cfg(not(windows))]
    {
        if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(x).join("frpc-gui")
        } else {
            home().join(".config").join("frpc-gui")
        }
    }
}

#[cfg_attr(windows, allow(dead_code))]
pub fn data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        exe_dir()
    }
    #[cfg(not(windows))]
    {
        if let Ok(x) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(x).join("frpc-gui")
        } else {
            home().join(".local").join("share").join("frpc-gui")
        }
    }
}

#[cfg(windows)]
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|s| s.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg_attr(windows, allow(dead_code))]
fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

pub fn config_path() -> PathBuf {
    #[cfg(windows)]
    {
        config_dir().join("frpc.toml")
    }
    #[cfg(not(windows))]
    {
        config_dir().join("frpc.toml")
    }
}

pub fn frpc_path() -> PathBuf {
    #[cfg(windows)]
    {
        exe_dir().join("frpc.exe")
    }
    #[cfg(not(windows))]
    {
        data_dir().join("bin").join("frpc")
    }
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = config_dir();
        fs::create_dir_all(&dir)?;
        fs::write(
            dir.join("frpc.toml"),
            toml::to_string_pretty(self).unwrap_or_default(),
        )
    }

    pub fn save_single(&self, name: &str) -> std::io::Result<PathBuf> {
        let dir = config_dir().join("proxies");
        fs::create_dir_all(&dir)?;
        let safe: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let mut one = self.clone();
        one.proxies.retain(|p| p.name == name);
        let path = dir.join(format!("{safe}.toml"));
        fs::write(&path, toml::to_string_pretty(&one).unwrap_or_default())?;
        Ok(path)
    }
}
