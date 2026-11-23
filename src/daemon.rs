use crate::config::Config;
use crate::runner::Runner;
use crate::TrayCmd;
use serde::Serialize;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};

/// 后端只监听本机回环，端口固定，绑不上就说明已经有实例
pub const PORT: u16 = 38917;

#[derive(Serialize)]
pub struct State {
    pub config: Config,
    pub running: Vec<String>,
    pub log: Vec<String>,
}

pub fn alive() -> bool {
    TcpStream::connect(("127.0.0.1", PORT)).is_ok()
}

pub fn current_exe() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "frpc-gui".into())
}

fn spawn_ui() {
    let _ = Command::new(current_exe())
        .arg("--ui")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn snapshot() -> State {
    let mut runner = Runner::new();
    State {
        config: Config::load(),
        running: runner.running_names(),
        log: runner.snapshot(),
    }
}

pub fn run() -> ! {
    let listener = match TcpListener::bind(("127.0.0.1", PORT)) {
        Ok(l) => l,
        Err(_) => {
            // 已经有后端在跑
            std::process::exit(0);
        }
    };

    let (tx, rx) = std::sync::mpsc::channel::<TrayCmd>();

    #[cfg(target_os = "linux")]
    let tray = crate::tray::spawn(tx);
    #[cfg(windows)]
    let tray = crate::tray_win::spawn(tx);
    #[cfg(not(any(target_os = "linux", windows)))]
    let tray = {
        let _ = tx;
        None::<()>
    };
    std::mem::forget(tray);

    std::thread::spawn(move || {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                TrayCmd::Show => spawn_ui(),
                TrayCmd::StartAll => {
                    let cfg = Config::load();
                    let mut runner = Runner::new();
                    for p in &cfg.proxies {
                        let _ = runner.start(&cfg, &p.name);
                    }
                }
                TrayCmd::StopAll => Runner::new().stop_all(),
                TrayCmd::Quit => std::process::exit(0),
            }
        }
    });

    for stream in listener.incoming().flatten() {
        std::thread::spawn(move || handle(stream));
    }
    std::process::exit(0);
}

fn handle(stream: TcpStream) {
    let mut writer = match stream.try_clone() {
        Ok(w) => w,
        Err(_) => return,
    };
    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        let reply = match line.trim() {
            "STATE" => match serde_json::to_string(&snapshot()) {
                Ok(json) => json,
                Err(e) => format!("ERR {e}"),
            },
            "STARTALL" => {
                let cfg = Config::load();
                let mut runner = Runner::new();
                for p in &cfg.proxies {
                    let _ = runner.start(&cfg, &p.name);
                }
                "OK".into()
            }
            "STOPALL" => {
                Runner::new().stop_all();
                "OK".into()
            }
            "SHOW" => {
                spawn_ui();
                "OK".into()
            }
            "QUIT" => std::process::exit(0),
            other => {
                let mut parts = other.splitn(2, ' ');
                match (parts.next(), parts.next()) {
                    (Some("START"), Some(name)) => {
                        let cfg = Config::load();
                        match Runner::new().start(&cfg, name) {
                            Ok(()) => "OK".into(),
                            Err(e) => format!("ERR {e}"),
                        }
                    }
                    (Some("STOP"), Some(name)) => {
                        Runner::new().stop(name);
                        "OK".into()
                    }
                    _ => "ERR unknown".into(),
                }
            }
        };
        if writeln!(writer, "{reply}").is_err() {
            break;
        }
        let _ = writer.flush();
    }
}
