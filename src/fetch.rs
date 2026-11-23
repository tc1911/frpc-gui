use crate::config::frpc_path;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub const VERSION: &str = "0.72.0";

pub fn platform_asset() -> Option<(&'static str, &'static str)> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(("linux_amd64", "tar.gz"))
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some(("linux_arm64", "tar.gz"))
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(("darwin_arm64", "tar.gz"))
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some(("darwin_amd64", "tar.gz"))
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some(("windows_amd64", "zip"))
    } else {
        None
    }
}

fn url_for(version: &str) -> Option<String> {
    let (os, ext) = platform_asset()?;
    Some(format!(
        "https://github.com/fatedier/frp/releases/download/v{version}/frp_{version}_{os}.{ext}"
    ))
}

fn push_line(log: &Arc<Mutex<Vec<String>>>, line: String) {
    let mut g = log.lock().unwrap();
    if g.len() >= crate::runner::MAX_LINES {
        g.remove(0);
    }
    g.push(line);
}

pub fn download_frpc(log: Arc<Mutex<Vec<String>>>, done: impl Fn(bool) + Send + 'static) {
    std::thread::spawn(move || {
        let ok = match try_download(&log) {
            Ok(()) => {
                push_line(&log, format!("frpc 已就绪 -> {}", frpc_path().display()));
                true
            }
            Err(e) => {
                push_line(&log, format!("下载失败: {e}"));
                false
            }
        };
        done(ok);
    });
}

fn try_download(log: &Arc<Mutex<Vec<String>>>) -> Result<(), String> {
    let url = url_for(VERSION).ok_or("当前架构没有官方预编译包")?;
    push_line(log, format!("下载 {url}"));

    let resp = ureq::get(&url)
        .call()
        .map_err(|e| format!("网络错误: {e}"))?;
    let mut body = Vec::new();
    resp.into_reader()
        .read_to_end(&mut body)
        .map_err(|e| format!("读取失败: {e}"))?;
    push_line(log, format!("收到 {} KB", body.len() / 1024));

    let bin_name = if cfg!(windows) { "frpc.exe" } else { "frpc" };
    let dest = frpc_path();
    if let Some(p) = dest.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }

    if url.ends_with(".tar.gz") {
        let gz = flate2::read::GzDecoder::new(&body[..]);
        let mut ar = tar::Archive::new(gz);
        let entries = ar.entries().map_err(|e| e.to_string())?;
        for entry in entries {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?.to_path_buf();
            if path.file_name().and_then(|s| s.to_str()) == Some(bin_name) {
                entry.unpack(&dest).map_err(|e| e.to_string())?;
                break;
            }
        }
    } else {
        extract_zip(&body, &dest, bin_name)?;
    }

    if !dest.exists() {
        return Err("压缩包里没找到 frpc".into());
    }
    set_exec(&dest)?;
    Ok(())
}

fn set_exec(p: &Path) -> Result<(), String> {
    #[cfg(not(unix))]
    let _ = p;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(p)
            .map_err(|e| e.to_string())?
            .permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(p, perm).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
fn extract_zip(body: &[u8], dest: &Path, bin_name: &str) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(body)).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.name().ends_with(bin_name) {
            let mut out = std::fs::File::create(dest).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut out).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err("压缩包里没找到 frpc".into())
}

#[cfg(not(windows))]
fn extract_zip(_body: &[u8], _dest: &Path, _bin_name: &str) -> Result<(), String> {
    Err("这个平台不走 zip".into())
}
