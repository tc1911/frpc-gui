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
            proxies: Vec::new(),
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

pub fn ipc_key_path() -> PathBuf {
    config_dir().join("ipc.key")
}

/// 本机 IPC 的口令，首次使用时生成，之后固定
pub fn ipc_key() -> String {
    let path = ipc_key_path();
    if let Ok(text) = fs::read_to_string(&path) {
        let key = text.trim().to_string();
        if key.len() >= 32 {
            return key;
        }
    }

    let mut raw = [0u8; 32];
    if getrandom::fill(&mut raw).is_err() {
        // 极端情况下退化成时间戳，至少不是可预测的空口令
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        raw[..16].copy_from_slice(&now.to_le_bytes());
        raw[16..].copy_from_slice(&std::process::id().to_le_bytes());
    }
    let key: String = raw.iter().map(|b| format!("{b:02x}")).collect();

    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(&path, &key);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    key
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
