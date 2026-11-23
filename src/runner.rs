use crate::config::frpc_path;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub const MAX_LINES: usize = 400;

pub struct Runner {
    child: Option<Child>,
    pub log: Arc<Mutex<Vec<String>>>,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            child: None,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn running(&mut self) -> bool {
        match self.child.as_mut() {
            Some(c) => match c.try_wait() {
                Ok(None) => true,
                _ => {
                    self.child = None;
                    false
                }
            },
            None => false,
        }
    }

    pub fn start(&mut self, cfg_path: &std::path::Path) -> std::io::Result<()> {
        self.stop();
        let bin = frpc_path();
        if !bin.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("frpc 不存在: {}", bin.display()),
            ));
        }
        self.push(format!("启动 {}", bin.display()));
        let mut child = Command::new(&bin)
            .arg("-c")
            .arg(cfg_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        for stream in [
            child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
            child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let log = self.log.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stream);
                for line in reader.lines().map_while(Result::ok) {
                    let mut g = log.lock().unwrap();
                    if g.len() >= MAX_LINES {
                        g.remove(0);
                    }
                    g.push(line);
                }
            });
        }
        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
            self.push("已停止".into());
        }
    }

    pub fn push(&self, line: String) {
        let line = strip_ansi(&line);
        let mut g = self.log.lock().unwrap();
        if g.len() >= MAX_LINES {
            g.remove(0);
        }
        g.push(line);
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.stop();
    }
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
