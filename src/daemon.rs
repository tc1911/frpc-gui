use crate::config::Config;
use crate::runner::Runner;
use crate::TrayCmd;
use serde::Serialize;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::{OnceLock, mpsc::Sender};

/// 后端只监听本机回环，端口固定，绑不上就说明已经有实例
pub const PORT: u16 = 38917;

#[derive(Serialize)]
pub struct State {
    pub running: Vec<String>,
    pub log: Vec<String>,
}

pub enum Job {
    Start(String),
    Stop(String),
    StartAll,
    StopAll,
}

static JOB_TX: OnceLock<Sender<Job>> = OnceLock::new();

fn start_worker() {
    let (tx, rx) = std::sync::mpsc::channel::<Job>();
    let _ = JOB_TX.set(tx);
    // 常驻线程：frpc 都由它拉起，PDEATHSIG 才有意义
    std::thread::spawn(move || {
        while let Ok(job) = rx.recv() {
            match job {
                Job::Start(name) => {
                    let cfg = Config::load();
                    if let Err(e) = Runner::new().start(&cfg, &name) {
                        Runner::new().note(format!("[{name}] 启动失败: {e}"));
                    }
                }
                Job::Stop(name) => Runner::new().stop(&name),
                Job::StartAll => {
                    let cfg = Config::load();
                    if cfg.proxies.is_empty() {
                        Runner::new().note("没有可启动的映射".into());
                    }
                    let mut runner = Runner::new();
                    for p in &cfg.proxies {
                        if let Err(e) = runner.start(&cfg, &p.name) {
                            runner.note(format!("[{}] 启动失败: {e}", p.name));
                        }
                    }
                }
                Job::StopAll => Runner::new().stop_all(),
            }
        }
    });
}

fn post(job: Job) {
    if let Some(tx) = JOB_TX.get() {
        let _ = tx.send(job);
    }
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

    start_worker();

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
                TrayCmd::StartAll => post(Job::StartAll),
                TrayCmd::StopAll => post(Job::StopAll),
                TrayCmd::Quit => {
                    Runner::new().stop_all();
                    crate::exit_ui::kill_ui();
                    std::process::exit(0)
                }
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
    let mut reader = BufReader::new(stream);

    // 第一行必须是 AUTH <口令>，本机别的进程猜不到
    let mut first = String::new();
    if reader.read_line(&mut first).is_err() {
        return;
    }
    let want = crate::config::ipc_key();
    let ok = first
        .trim()
        .strip_prefix("AUTH ")
        .map(|k| k == want)
        .unwrap_or(false);
    if !ok {
        let _ = writeln!(writer, "ERR auth");
        let _ = writer.flush();
        return;
    }
    let _ = writeln!(writer, "OK");
    let _ = writer.flush();

    for line in reader.lines().map_while(Result::ok) {
        let reply = match line.trim() {
            "STATE" => match serde_json::to_string(&snapshot()) {
                Ok(json) => json,
                Err(e) => format!("ERR {e}"),
            },
            "STARTALL" => {
                post(Job::StartAll);
                "OK".into()
            }
            "STOPALL" => {
                post(Job::StopAll);
                "OK".into()
            }
            "SHOW" => {
                spawn_ui();
                "OK".into()
            }
            "QUIT" => {
                Runner::new().stop_all();
                crate::exit_ui::kill_ui();
                std::process::exit(0)
            }
            other => {
                let mut parts = other.splitn(2, ' ');
                match (parts.next(), parts.next()) {
                    (Some("START"), Some(name)) => {
                        post(Job::Start(name.to_string()));
                        "OK".into()
                    }
                    (Some("STOP"), Some(name)) => {
                        post(Job::Stop(name.to_string()));
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
