/// 后端退出时把界面进程一并收掉，不靠 IPC 断线去猜
#[cfg(target_os = "linux")]
pub fn kill_ui() {
    let me = std::process::id();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if pid == me {
            continue;
        }
        let Ok(raw) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
            continue;
        };
        let args: Vec<String> = raw
            .split(|b| *b == 0)
            .map(|s| String::from_utf8_lossy(s).to_string())
            .collect();
        if args.iter().any(|a| a == "--daemon") {
            continue;
        }
        let Some(first) = args.first() else { continue };
        let name = std::path::Path::new(first)
            .file_name()
            .map(|s| s.to_string_lossy().to_string());
        if name.as_deref() == Some("frpc-gui") {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
    }
}

#[cfg(windows)]
pub fn kill_ui() {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

    let me = std::process::id();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let raw: String = entry
                    .szExeFile
                    .iter()
                    .take_while(|c| **c != 0)
                    .map(|c| *c as u8 as char)
                    .collect();
                if entry.th32ProcessID != me
                    && entry.th32ProcessID != 0
                    && raw.eq_ignore_ascii_case("frpc-gui.exe")
                {
                    let h = OpenProcess(PROCESS_TERMINATE, 0, entry.th32ProcessID);
                    if !h.is_null() {
                        TerminateProcess(h, 0);
                        CloseHandle(h);
                    }
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn kill_ui() {}
