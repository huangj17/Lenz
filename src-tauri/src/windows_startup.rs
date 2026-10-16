//! Windows GUI 构建没有可见 stderr;为启动期错误保留独立于 Tauri 的本地记录。
use std::io::Write;

pub fn install_error_handler() {
    let main_thread = std::thread::current().id();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        // 后台任务的 panic 仍由现有日志处理,避免打断正在使用的窗口。
        if std::thread::current().id() != main_thread {
            return;
        }
        let message = info.to_string();
        let directory = std::env::var_os("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("app.lenz.desktop/logs");
        let path = directory.join("startup-error.log");
        let written = std::fs::create_dir_all(&directory)
            .and_then(|()| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
            })
            .and_then(|mut file| writeln!(file, "{:?}\n{message}\n", std::time::SystemTime::now()));
        // 无人值守的原生启动测试仍记录错误,通过 stderr / 退出码判定失败。
        if std::env::var_os("LENZ_STARTUP_NO_DIALOG").is_some() {
            return;
        }
        let details = if written.is_ok() {
            format!("日志 / Log: {}", path.display())
        } else {
            "无法写入启动日志 / Could not write the startup log.".into()
        };
        let text: Vec<u16> =
            format!("Lenz 启动失败 / Lenz could not start.\n\n{message}\n\n{details}")
                .encode_utf16()
                .chain(Some(0))
                .collect();
        use windows::core::{w, PCWSTR};
        use windows::Win32::UI::WindowsAndMessaging::{
            MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND,
        };
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                w!("Lenz"),
                MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
            );
        }
    }));
}
