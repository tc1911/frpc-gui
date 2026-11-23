# frpc-gui

套在 [frpc](https://github.com/fatedier/frp) 外面的图形前端，用 [GPUI](https://gpui.rs) 写的。

主页只负责启停和看日志，设置页放服务器地址、token、端口映射。每条映射起一个独立的 frpc 进程，可以单独启停，也可以一键全启。

## 进程结构

程序分前后端两个角色，同一个可执行文件：

- 后端 `frpc-gui --daemon`：常驻进程，管 frpc、托盘，监听本机 `127.0.0.1:38917` 接控制指令
- 前端：窗口，只做展示，启停通过本机 IPC 交给后端

所以关掉窗口只是关了界面，frpc 和托盘都还在。托盘点「显示窗口」或者发一条 `SHOW` 上去，会重新拉起界面进程。托盘的「退出」才是真的把后端关掉。

直接运行 `frpc-gui` 时，如果后端没起会自动在后台拉一个。

## 特性

- frpc 首次启动时检测，缺了才弹窗，从官方 release 下载到程序目录
- 每条映射独立进程，单条启停
- 服务器地址和 token 默认掩码，旁边按钮切换
- 保存按钮跟着改动状态亮灭
- 托盘菜单：全部启动 / 全部停止 / 退出
  - Linux 走 StatusNotifierItem
  - Windows 走 Shell_NotifyIcon
- 关界面不退后端

## 构建

需要 Rust 1.85+。

```
cargo build --release
```

Linux 还要有 wayland、x11、libxkbcommon 的开发包，日常装 niri 或 GNOME 的机器一般都有。

Windows 上直接 `cargo build --release` 就能出 exe。要安装包的话装个 Inno Setup，然后：

```
iscc packaging/windows/installer.iss
```

安装到 `%LOCALAPPDATA%\Programs\frpc-gui`，不需要管理员权限。

## 配置放在哪

- Linux：`~/.config/frpc-gui/frpc.toml`
- Windows：exe 同目录的 `frpc.toml`

单条映射启动时会另外生成 `proxies/<名字>.toml`，一份配置只带那条映射。

## 关于 vendor

gpui 0.2.2 的依赖链里有两个包在新版 Rust 上有问题，`vendor/` 下各放了一份改过的：

- `xattr` 0.2.3 在 Linux 分支引用了只在 BSD 存在的 `ENOATTR`
- `proc-macro-error2` 2.0.1 把私有的 `extern crate proc_macro` 拿去 re-export

`Cargo.toml` 里用 `[patch.crates-io]` 指过去。gpui 上游修了之前别删。

## License

MIT
