#![cfg_attr(windows, windows_subsystem = "windows")]

mod config;
mod fetch;
mod runner;
mod text_input;

#[cfg(target_os = "linux")]
mod tray;

#[cfg(windows)]
mod tray_win;

use config::{Auth, Config, Proxy, config_path, frpc_path};
use gpui::prelude::*;
use gpui::*;
use runner::Runner;
use std::time::Duration;
use text_input::{
    Backspace, Copy, Cut, Delete, End, Home, Left, Paste, Right, SelectAll, SelectLeft,
    SelectRight, ShowCharacterPalette, TextInput,
};

#[derive(Clone, Copy, Debug)]
pub enum TrayCmd {
    StartAll,
    StopAll,
    Quit,
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Settings,
}

struct Root {
    server_addr: Entity<TextInput>,
    server_port: Entity<TextInput>,
    token: Entity<TextInput>,
    proxies: Vec<ProxyRow>,
    runner: Runner,
    status: &'static str,
    show_download: bool,
    downloading: bool,
    page: Page,
    saved: Config,
    dirty: bool,
    #[cfg(any(target_os = "linux", windows))]
    tray: TrayState,
}

#[cfg(target_os = "linux")]
struct TrayState {
    rx: std::sync::mpsc::Receiver<TrayCmd>,
    _handle: Option<tray::TrayHandle>,
}

#[cfg(windows)]
struct TrayState {
    rx: std::sync::mpsc::Receiver<TrayCmd>,
    _handle: Option<tray_win::TrayHandle>,
}

struct ProxyRow {
    name: Entity<TextInput>,
    local_ip: Entity<TextInput>,
    local_port: Entity<TextInput>,
    remote_port: Entity<TextInput>,
}

impl ProxyRow {
    fn new(cx: &mut App, p: &Proxy) -> Self {
        Self {
            name: cx.new(|cx| TextInput::new(cx, p.name.clone(), "名称", false, false)),
            local_ip: cx
                .new(|cx| TextInput::new(cx, p.local_ip.clone(), "127.0.0.1", false, false)),
            local_port: cx
                .new(|cx| TextInput::new(cx, p.local_port.to_string(), "25565", true, false)),
            remote_port: cx
                .new(|cx| TextInput::new(cx, p.remote_port.to_string(), "25565", true, false)),
        }
    }
}

impl Root {
    fn new(cx: &mut Context<Self>) -> Self {
        let cfg = Config::load();
        let show_download = !frpc_path().exists();
        #[cfg(target_os = "linux")]
        let tray_state = {
            let (tx, rx) = std::sync::mpsc::channel();
            let handle = tray::spawn(tx);
            TrayState {
                rx,
                _handle: handle,
            }
        };
        #[cfg(windows)]
        let tray_state = {
            let (tx, rx) = std::sync::mpsc::channel();
            let handle = tray_win::spawn(tx);
            TrayState {
                rx,
                _handle: handle,
            }
        };
        Self {
            server_addr: cx
                .new(|cx| TextInput::new(cx, cfg.server_addr.clone(), "服务器地址", false, true)),
            server_port: cx
                .new(|cx| TextInput::new(cx, cfg.server_port.to_string(), "7000", true, false)),
            token: cx
                .new(|cx| TextInput::new(cx, cfg.auth.token.clone(), "认证 token", false, true)),
            proxies: cfg.proxies.iter().map(|p| ProxyRow::new(cx, p)).collect(),
            runner: Runner::new(),
            status: "未启动",
            show_download,
            downloading: false,
            page: Page::Home,
            saved: cfg.clone(),
            dirty: false,
            #[cfg(any(target_os = "linux", windows))]
            tray: tray_state,
        }
    }

    fn collect(&self, cx: &App) -> Config {
        Config {
            server_addr: self.server_addr.read(cx).value(),
            server_port: self.server_port.read(cx).value().parse().unwrap_or(7000),
            auth: Auth {
                method: "token".into(),
                token: self.token.read(cx).value(),
            },
            proxies: self
                .proxies
                .iter()
                .map(|p| Proxy {
                    name: p.name.read(cx).value(),
                    kind: "tcp".into(),
                    local_ip: p.local_ip.read(cx).value(),
                    local_port: p.local_port.read(cx).value().parse().unwrap_or(25565),
                    remote_port: p.remote_port.read(cx).value().parse().unwrap_or(25565),
                })
                .collect(),
        }
    }

    fn tick(&mut self, cx: &App) {
        if self.downloading && frpc_path().exists() {
            self.downloading = false;
            self.show_download = false;
            self.runner.push("frpc 下载完成".into());
        }
        self.dirty = self.collect(cx) != self.saved;
        let running = self.runner.running_names().len();
        self.status = if running == 0 {
            "未启动"
        } else if running >= self.proxies.len() {
            "全部运行中"
        } else {
            "部分运行中"
        };
    }

    #[cfg(target_os = "linux")]
    fn pump_tray(&mut self, cx: &mut Context<Self>) {
        while let Ok(cmd) = self.tray.rx.try_recv() {
            match cmd {
                TrayCmd::StartAll => self.start_all(cx),
                TrayCmd::StopAll => self.stop_all(cx),
                TrayCmd::Quit => {
                    cx.quit();
                    return;
                }
            }
        }
    }

    #[cfg(windows)]
    fn pump_tray(&mut self, cx: &mut Context<Self>) {
        while let Ok(cmd) = self.tray.rx.try_recv() {
            match cmd {
                TrayCmd::StartAll => self.start_all(cx),
                TrayCmd::StopAll => self.stop_all(cx),
                TrayCmd::Quit => {
                    cx.quit();
                    return;
                }
            }
        }
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    fn pump_tray(&mut self, _cx: &mut Context<Self>) {}

    fn start_one(&mut self, name: String, cx: &mut Context<Self>) {
        if !frpc_path().exists() {
            self.show_download = true;
            cx.notify();
            return;
        }
        let cfg = self.collect(cx);
        let _ = cfg.save();
        if cfg.server_addr.trim().is_empty() {
            self.runner.push("服务器地址还没填".into());
            self.page = Page::Settings;
            cx.notify();
            return;
        }
        if let Err(e) = self.runner.start(&cfg, &name) {
            self.runner.push(format!("[{name}] 启动失败: {e}"));
        }
        self.tick(cx);
        cx.notify();
    }

    fn start_all(&mut self, cx: &mut Context<Self>) {
        if !frpc_path().exists() {
            self.show_download = true;
            cx.notify();
            return;
        }
        let cfg = self.collect(cx);
        let _ = cfg.save();
        if cfg.server_addr.trim().is_empty() {
            self.runner.push("服务器地址还没填".into());
            self.page = Page::Settings;
            cx.notify();
            return;
        }
        let names: Vec<String> = cfg.proxies.iter().map(|p| p.name.clone()).collect();
        self.runner
            .push(format!("-> {}:{}", cfg.server_addr, cfg.server_port));
        for name in names {
            if let Err(e) = self.runner.start(&cfg, &name) {
                self.runner.push(format!("[{name}] 启动失败: {e}"));
            }
        }
        self.tick(cx);
        cx.notify();
    }

    fn stop_one(&mut self, name: String, cx: &mut Context<Self>) {
        self.runner.stop(&name);
        self.tick(cx);
        cx.notify();
    }

    fn stop_all(&mut self, cx: &mut Context<Self>) {
        self.runner.stop_all();
        self.tick(cx);
        cx.notify();
    }

    fn download(&mut self, cx: &mut Context<Self>) {
        if self.downloading {
            return;
        }
        self.downloading = true;
        self.runner.push("开始下载 frpc".into());
        fetch::download_frpc(self.runner.log.clone(), |_| {});
        cx.notify();
    }

    fn add_proxy(&mut self, cx: &mut Context<Self>) {
        let p = Proxy {
            name: format!("proxy{}", self.proxies.len() + 1),
            kind: "tcp".into(),
            local_ip: "127.0.0.1".into(),
            local_port: 25565,
            remote_port: 25565,
        };
        self.proxies.push(ProxyRow::new(cx, &p));
        cx.notify();
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let cfg = self.collect(cx);
        match cfg.save() {
            Ok(()) => {
                self.saved = cfg;
                self.dirty = false;
                self.runner.push("配置已保存".into());
            }
            Err(e) => self.runner.push(format!("保存失败: {e}")),
        }
        cx.notify();
    }

    fn render_home(&mut self, cx: &mut Context<Self>) -> Div {
        let running = self.status != "未启动";
        let has_bin = frpc_path().exists();
        let active = self.runner.running_names();
        let lines = self.runner.snapshot();
        let tail: Vec<SharedString> = lines
            .iter()
            .rev()
            .take(24)
            .rev()
            .map(|s| SharedString::from(s.clone()))
            .collect();

        let cfg = self.collect(cx);
        let server = if cfg.server_addr.is_empty() {
            "还没设置服务器".to_string()
        } else {
            format!("{}:{}", cfg.server_addr, cfg.server_port)
        };

        let rows: Vec<Div> = cfg
            .proxies
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let on = active.iter().any(|n| n == &p.name);
                let name = p.name.clone();
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(0x27272a))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(div().size_2().rounded_full().bg(rgb(if on {
                                0x22c55e
                            } else {
                                0x52525b
                            })))
                            .child(div().text_sm().child(p.name.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(rgb(0xa1a1aa))
                                    .child(format!(
                                        "{}:{}  ->  {}",
                                        p.local_ip, p.local_port, p.remote_port
                                    )),
                            ),
                    )
                    .child(
                        div().flex().flex_row().gap_2().child(
                            styled_button(
                                SharedString::from(format!("p{}", i)),
                                if on { "停止" } else { "启动" },
                                if on { 0xdc2626 } else { 0x2563eb },
                                true,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    if on {
                                        this.stop_one(name.clone(), cx);
                                    } else {
                                        this.start_one(name.clone(), cx);
                                    }
                                },
                            )),
                        ),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_row()
            .size_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(400.))
                    .h_full()
                    .p_4()
                    .gap_3()
                    .border_r_1()
                    .border_color(rgb(0x27272a))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .child(div().size_2().rounded_full().bg(rgb(if running {
                                        0x22c55e
                                    } else {
                                        0x71717a
                                    })))
                                    .child(div().text_base().child(if running {
                                        "运行中"
                                    } else {
                                        "未启动"
                                    })),
                            )
                            .child(styled_button("settings", "设置", 0x3f3f46, true).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.page = Page::Settings;
                                    cx.notify();
                                }),
                            )),
                    )
                    .child(div().text_xs().text_color(rgb(0x71717a)).child(server))
                    .child(div().flex().flex_col().gap_2().children(rows))
                    .when(cfg.proxies.is_empty(), |d| {
                        d.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x52525b))
                                .child("还没有映射，去设置页加一条"),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                styled_button("start", "全部启动", 0x2563eb, !running)
                                    .on_click(cx.listener(|this, _, _, cx| this.start_all(cx))),
                            )
                            .child(
                                styled_button("stop", "全部停止", 0xdc2626, running)
                                    .on_click(cx.listener(|this, _, _, cx| this.stop_all(cx))),
                            ),
                    )
                    .child(div().flex_1())
                    .child(div().text_xs().text_color(rgb(0x52525b)).child(if has_bin {
                        format!("frpc {} · {} 个在跑", fetch::VERSION, active.len())
                    } else {
                        "frpc 未安装".to_string()
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .h_full()
                    .p_4()
                    .gap_1()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(rgb(0xa1a1aa))
                    .children(tail.into_iter().map(|l| div().child(l))),
            )
    }

    fn render_settings(&mut self, cx: &mut Context<Self>) -> Div {
        let has_bin = frpc_path().exists();
        let proxy_rows: Vec<Div> = self
            .proxies
            .iter()
            .enumerate()
            .map(|(i, p)| {
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_end()
                    .child(div().w(px(90.)).child(labelled("名称", p.name.clone())))
                    .child(
                        div()
                            .w(px(110.))
                            .child(labelled("本地地址", p.local_ip.clone())),
                    )
                    .child(
                        div()
                            .w(px(70.))
                            .child(labelled("本地端口", p.local_port.clone())),
                    )
                    .child(
                        div()
                            .w(px(70.))
                            .child(labelled("远端端口", p.remote_port.clone())),
                    )
                    .child(
                        styled_button(
                            SharedString::from(format!("del{}", i)),
                            "删",
                            0x3f3f46,
                            true,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if i < this.proxies.len() {
                                this.proxies.remove(i);
                                cx.notify();
                            }
                        })),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(
                        styled_button("back", "返回", 0x3f3f46, true).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.page = Page::Home;
                                cx.notify();
                            },
                        )),
                    )
                    .child(div().text_base().child("设置")),
            )
            .child(
                div()
                    .id("settings-body")
                    .flex()
                    .flex_col()
                    .gap_3()
                    .overflow_y_scroll()
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .items_end()
                                    .child(
                                        div().flex_1().child(labelled(
                                            "服务器地址",
                                            self.server_addr.clone(),
                                        )),
                                    )
                                    .child(
                                        styled_button(
                                            "eyeserver",
                                            if self.server_addr.read(cx).masked {
                                                "显示"
                                            } else {
                                                "隐藏"
                                            },
                                            0x3f3f46,
                                            true,
                                        )
                                        .on_click(
                                            cx.listener(|this, _, _, cx| {
                                                let addr = this.server_addr.clone();
                                                addr.update(cx, |input, cx| input.toggle_mask(cx));
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(100.))
                                    .child(labelled("端口", self.server_port.clone())),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .items_end()
                            .child(
                                div()
                                    .flex_1()
                                    .child(labelled("认证 token", self.token.clone())),
                            )
                            .child(
                                styled_button(
                                    "eye",
                                    if self.token.read(cx).masked {
                                        "显示"
                                    } else {
                                        "隐藏"
                                    },
                                    0x3f3f46,
                                    true,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        let token = this.token.clone();
                                        token.update(cx, |input, cx| input.toggle_mask(cx));
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(0x71717a))
                            .child("端口映射"),
                    )
                    .child(div().flex().flex_col().gap_2().children(proxy_rows))
                    .child(
                        styled_button("add", "+ 添加映射", 0x3f3f46, true)
                            .on_click(cx.listener(|this, _, _, cx| this.add_proxy(cx))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .mt_2()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0x1f1f23))
                            .child(div().text_xs().text_color(rgb(0xa1a1aa)).child(if has_bin {
                                format!("frpc {} 已就绪", fetch::VERSION)
                            } else {
                                "frpc 未安装".to_string()
                            }))
                            .child(
                                styled_button(
                                    "dl",
                                    if self.downloading {
                                        "下载中…"
                                    } else if has_bin {
                                        "重新下载"
                                    } else {
                                        "下载 frpc"
                                    },
                                    0x3f3f46,
                                    !self.downloading,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.download(cx))),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .mt_2()
                            .child(
                                styled_button("save", "保存", 0x2563eb, self.dirty)
                                    .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                            )
                            .child(
                                styled_button("rawfile", "打开配置文件", 0x3f3f46, true).on_click(
                                    cx.listener(|_, _, _, _| {
                                        let _ = std::fs::create_dir_all(config::config_dir());
                                        let p = config_path();
                                        if !p.exists() {
                                            let _ = Config::default().save();
                                        }
                                        open_path(&p);
                                    }),
                                ),
                            ),
                    ),
            )
    }

    fn render_download_modal(&mut self, cx: &mut Context<Self>) -> Div {
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x000000cc))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_6()
                    .rounded_lg()
                    .bg(rgb(0x27272a))
                    .w(px(360.))
                    .child(div().text_base().child("没有找到 frpc"))
                    .child(div().text_sm().text_color(rgb(0xa1a1aa)).child(format!(
                        "要从官方 GitHub 下载 v{} 吗？大约 15 MB。",
                        fetch::VERSION
                    )))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                styled_button(
                                    "modaldl",
                                    if self.downloading {
                                        "下载中…"
                                    } else {
                                        "下载"
                                    },
                                    0x2563eb,
                                    !self.downloading,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.download(cx))),
                            )
                            .child(styled_button("later", "稍后", 0x3f3f46, true).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.show_download = false;
                                    cx.notify();
                                }),
                            )),
                    ),
            )
    }
}

fn open_path(p: &std::path::Path) {
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    let _ = std::process::Command::new(cmd).arg(p).spawn();
}

fn styled_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    bg: u32,
    enabled: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_md()
        .bg(rgb(if enabled { bg } else { 0x3f3f46 }))
        .text_color(rgb(if enabled { 0xfafafa } else { 0x71717a }))
        .text_sm()
        .child(label.into())
        .when(enabled, |d| d.cursor_pointer())
}

fn labelled(label: &str, input: Entity<TextInput>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x71717a))
                .child(label.to_string()),
        )
        .child(input)
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.page {
            Page::Home => self.render_home(cx),
            Page::Settings => self.render_settings(cx),
        };
        let modal = if self.show_download {
            Some(self.render_download_modal(cx))
        } else {
            None
        };

        div()
            .relative()
            .size_full()
            .bg(rgb(0x18181b))
            .text_color(rgb(0xe4e4e7))
            .child(body)
            .children(modal)
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(880.), px(620.)), cx);
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| Root::new(cx)),
            )
            .expect("failed to open window");

        let _ = handle.update(cx, |_root, _window, cx| {
            cx.spawn(async move |this, cx| {
                loop {
                    let alive = this
                        .update(cx, |app, cx| {
                            app.tick(cx);
                            app.pump_tray(cx);
                            cx.notify();
                        })
                        .is_ok();
                    if !alive {
                        break;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(600))
                        .await;
                }
            })
            .detach();
        });

        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("shift-left", SelectLeft, None),
            KeyBinding::new("shift-right", SelectRight, None),
            KeyBinding::new("ctrl-a", SelectAll, None),
            KeyBinding::new("ctrl-v", Paste, None),
            KeyBinding::new("ctrl-c", Copy, None),
            KeyBinding::new("ctrl-x", Cut, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("ctrl-shift-u", ShowCharacterPalette, None),
        ]);
        cx.activate(true);
    });
}
