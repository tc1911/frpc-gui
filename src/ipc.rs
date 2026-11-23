use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Deserialize, Clone, Default)]
pub struct State {
    pub running: Vec<String>,
    pub log: Vec<String>,
}

pub struct Ipc {
    state: Arc<Mutex<State>>,
    tx: Sender<String>,
    #[allow(dead_code)]
    online: Arc<Mutex<bool>>,
}

impl Ipc {
    pub fn connect(port: u16) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let online = Arc::new(Mutex::new(false));
        let (tx, rx) = channel::<String>();

        let s2 = state.clone();
        let o2 = online.clone();
        std::thread::spawn(move || {
            loop {
                match TcpStream::connect(("127.0.0.1", port)) {
                    Ok(stream) => {
                        *o2.lock().unwrap() = true;
                        stream
                            .set_read_timeout(Some(Duration::from_millis(1200)))
                            .ok();
                        let mut writer = match stream.try_clone() {
                            Ok(w) => w,
                            Err(_) => {
                                std::thread::sleep(Duration::from_millis(500));
                                continue;
                            }
                        };
                        let _ = writeln!(writer, "STATE");
                        let _ = writer.flush();
                        let mut line = String::new();
                        if BufReader::new(stream).read_line(&mut line).is_ok() {
                            if let Ok(st) = serde_json::from_str::<State>(line.trim()) {
                                *s2.lock().unwrap() = st;
                            }
                        }
                    }
                    Err(_) => {
                        *o2.lock().unwrap() = false;
                    }
                }

                while let Ok(cmd) = rx.try_recv() {
                    if let Ok(mut s) = TcpStream::connect(("127.0.0.1", port)) {
                        let _ = writeln!(s, "{cmd}");
                        let _ = s.flush();
                    }
                }
                std::thread::sleep(Duration::from_millis(450));
            }
        });

        Self {
            state,
            tx,
            online,
        }
    }

    pub fn send(&self, cmd: String) {
        let _ = self.tx.send(cmd);
    }

    pub fn state(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    #[allow(dead_code)]
    pub fn online(&self) -> bool {
        *self.online.lock().unwrap()
    }
}
