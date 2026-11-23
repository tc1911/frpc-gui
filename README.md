# frpc-gui

套在 [frpc](https://github.com/fatedier/frp) 外面的图形前端，用 [GPUI](https://gpui.rs) 写的。

主页只负责启停和看日志，设置页放服务器地址、token、端口映射。每条映射起一个独立的 frpc 进程，可以单独启停，也可以一键全启。

## 进程结构

程序分前后端两个角色，同一个可执行文件：

- 后端 `frpc-gui --daemon`：常驻进程，管 frpc 和托盘，监听本机 `127.0.0.1:38917` 接控制指令
- 前端：窗口，只做展示，启停通过本机 IPC 交给后端

所以关掉窗口只是关了界面，frpc 和托盘都还在。托盘点「显示窗口」或者发一条 `SHOW` 上去，会重新拉起界面进程。托盘的「退出」才是真的把后端关掉。

直接运行 `frpc-gui` 时，如果后端没起会自动在后台拉一个。

## 本机通信

前后端走 TCP，只绑 `127.0.0.1`，不是 `0.0.0.0`。局域网或者外部地址连过来内核直接拒绝，到不了应用层。

连接后第一行必须发 `AUTH <口令>`，口令是 32 字节随机数，存在 `~/.config/frpc-gui/ipc.key`，权限 0600。对不上就回 `ERR auth` 然后断开。

状态快照（`STATE`）只返回运行中的映射名和日志尾巴，不含 token —— 认证信息不会跟着状态一起出去。

信任边界是当前用户：同机其他用户读不到 key，同用户的进程能读，这个挡不住。

可用的指令：

```
AUTH <key>
STATE        # 运行列表 + 日志
START <名字>  # 启一条
STOP <名字>
STARTALL
STOPALL
SHOW         # 拉起界面
QUIT         # 停掉所有隧道并退出后端
```

## 特性

- frpc 首次启动时检测，缺了才弹窗，从官方 release 下载到程序目录
- 下载完比对官方 `frp_sha256_checksums.txt`，对不上不落盘
- 每条映射独立进程，单条启停
- 服务器地址和 token 默认掩码，旁边按钮切换
- 保存按钮跟着改动状态亮灭
- 托盘菜单：全部启动 / 全部停止 / 退出
  - Linux 走 StatusNotifierItem
  - Windows 走 Shell_NotifyIcon
- 关界面不退后端
- 启动失败、frpc 进程退出都会写进日志，不是默默没反应

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

Linux 的 deb 和 rpm 有现成脚本，都从 git tag 拉源码构建：

```
./packaging/linux/build-deb.sh
./packaging/linux/build-rpm.sh
```

Arch 用 `PKGBUILD`（配方在 [tc191-pkgs](https://github.com/tc1911/tc191-pkgs)）。

## 配置放在哪

- Linux：`~/.config/frpc-gui/frpc.toml`
- Windows：exe 同目录的 `frpc.toml`

单条映射启动时会另外生成 `proxies/<名字>.toml`，一份配置只带那条映射。

Linux 上配置文件写入后会被设成 0600，里面存的是 token。

frpc 二进制不在系统路径里，Linux 在 `~/.local/share/frpc-gui/bin/frpc`，Windows 跟 exe 放一起。

## 已知限制

- 关窗口退到托盘只在 Windows 上是真隐藏；gpui 的 wayland 后端在窗口全关时会停掉事件循环，Linux 上「关界面」就是关掉这个前端进程，后端和隧道还是一样留着
- 映射类型目前只有 tcp，UDP 得手动改配置

## 关于 vendor

gpui 0.2.2 的依赖链里有两个包在新版 Rust 上有问题，`vendor/` 下各放了一份改过的：

- `xattr` 0.2.3 在 Linux 分支引用了只在 BSD 存在的 `ENOATTR`
- `proc-macro-error2` 2.0.1 把私有的 `extern crate proc_macro` 拿去 re-export

`Cargo.toml` 里用 `[patch.crates-io]` 指过去。gpui 上游修了之前别删。

## License

MIT
