use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(rename = "serverAddr")]
    pub server_addr: String,
    #[serde(rename = "serverPort")]
    pub server_port: u16,
    pub auth: Auth,
    #[serde(default)]
    pub proxies: Vec<Proxy>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Auth {
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(x).join("frpc-gui")
    } else {
        home().join(".config").join("frpc-gui")
    }
}

pub fn data_dir() -> PathBuf {
    if let Ok(x) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(x).join("frpc-gui")
    } else {
        home().join(".local").join("share").join("frpc-gui")
    }
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

pub fn config_path() -> PathBuf {
    config_dir().join("frpc.toml")
}

pub fn frpc_path() -> PathBuf {
    let name = if cfg!(windows) { "frpc.exe" } else { "frpc" };
    data_dir().join("bin").join(name)
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_default(),
            Err(_) => {
                let cfg = Config::default();
                let _ = cfg.save();
                cfg
            }
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = config_dir();
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("frpc.toml"), toml::to_string_pretty(self).unwrap_or_default())
    }
}
