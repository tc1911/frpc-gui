use crate::config::{frpc_path, Config};
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub const MAX_LINES: usize = 400;

struct Proc {
    name: String,
    child: Child,
}

pub struct Runner {
    procs: Vec<Proc>,
    pub log: Arc<Mutex<Vec<String>>>,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            procs: Vec::new(),
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn reap(&mut self) {
        self.procs
            .retain_mut(|p| matches!(p.child.try_wait(), Ok(None)));
    }

    pub fn is_running(&mut self, name: &str) -> bool {
        self.reap();
        self.procs.iter().any(|p| p.name == name)
    }

    pub fn running_names(&mut self) -> Vec<String> {
        self.reap();
        self.procs.iter().map(|p| p.name.clone()).collect()
    }

    pub fn start(&mut self, cfg: &Config, name: &str) -> std::io::Result<()> {
        if self.is_running(name) {
            return Ok(());
        }
        let bin = frpc_path();
        if !bin.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("frpc 不存在: {}", bin.display()),
            ));
        }
        let path = cfg.save_single(name)?;

        let mut child = Command::new(&bin)
            .arg("-c")
            .arg(&path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut readers: Vec<Box<dyn Read + Send>> = Vec::new();
        if let Some(o) = child.stdout.take() {
            readers.push(Box::new(o));
        }
        if let Some(e) = child.stderr.take() {
            readers.push(Box::new(e));
        }
        for stream in readers {
            let log = self.log.clone();
            let tag = name.to_string();
            thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    push_line(&log, format!("[{tag}] {line}"));
                }
            });
        }

        self.procs.push(Proc {
            name: name.to_string(),
            child,
        });
        push_line(&self.log, format!("[{name}] 已启动"));
        Ok(())
    }

    pub fn stop(&mut self, name: &str) {
        let mut hit = false;
        self.procs.retain_mut(|p| {
            if p.name != name {
                return true;
            }
            let _ = p.child.kill();
            let _ = p.child.wait();
            hit = true;
            false
        });
        if hit {
            push_line(&self.log, format!("[{name}] 已停止"));
        }
    }

    pub fn stop_all(&mut self) {
        for p in self.procs.iter_mut() {
            let _ = p.child.kill();
            let _ = p.child.wait();
        }
        if !self.procs.is_empty() {
            push_line(&self.log, "全部已停止".into());
        }
        self.procs.clear();
    }

    pub fn push(&self, line: String) {
        push_line(&self.log, line);
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn push_line(log: &Arc<Mutex<Vec<String>>>, line: String) {
    let line = strip_ansi(&line);
    let mut g = log.lock().unwrap();
    if g.len() >= MAX_LINES {
        g.remove(0);
    }
    g.push(line);
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut iter = s.chars().peekable();
    while let Some(c) = iter.next() {
        if c == '\u{1b}' && iter.peek() == Some(&'[') {
            iter.next();
            for c2 in iter.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}
