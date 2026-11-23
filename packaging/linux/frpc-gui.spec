%global debug_package %{nil}

Name:           frpc-gui
Version:        0.1.0
Release:        1
Summary:        frpc 的图形前端（GPUI）
License:        MIT
URL:            https://github.com/tc1911/frpc-gui
Source0:        %{name}-%{version}.tar.gz

# 按 soname 写，dnf/zypper 才解析得出来
Requires:       libc.so.6()(64bit)
Requires:       libgcc_s.so.1()(64bit)
Requires:       libXau.so.6()(64bit)
Requires:       libxcb.so.1()(64bit)
Requires:       libXdmcp.so.6()(64bit)
Requires:       libxkbcommon.so.0()(64bit)
Requires:       libxkbcommon-x11.so.0()(64bit)

%description
套在 frpc 外面的图形前端，用 GPUI 写的。

主页负责启停隧道和看日志，设置页放服务器地址、token 和端口映射。
每条映射起一个独立的 frpc 进程，可以单独启停。

程序分前后端：守护进程常驻，管 frpc 和托盘；界面只是展示层，
关掉界面不影响已经建立的隧道。

%prep
%setup -q

%build
export RUSTUP_TOOLCHAIN=stable
cargo build --release --locked

%install
install -Dm755 target/release/frpc-gui %{buildroot}%{_bindir}/frpc-gui
install -Dm644 packaging/linux/frpc-gui.desktop \
    %{buildroot}%{_datadir}/applications/frpc-gui.desktop
install -Dm644 assets/icon.png \
    %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/frpc-gui.png
install -Dm644 assets/icon.svg \
    %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/frpc-gui.svg
install -Dm644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE

%files
%{_bindir}/frpc-gui
%{_datadir}/applications/frpc-gui.desktop
%{_datadir}/icons/hicolor/512x512/apps/frpc-gui.png
%{_datadir}/icons/hicolor/scalable/apps/frpc-gui.svg
%{_licensedir}/%{name}/LICENSE

%changelog
* Tue Nov 24 2026 tc191 <tc191_dev@163.com> - 0.1.0-1
- 首个版本
