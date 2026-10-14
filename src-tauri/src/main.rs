// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "windows")]
    {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args
            .first()
            .is_some_and(|arg| arg == "--diagnose-windows-ocr")
        {
            let success = args.len() == 2
                && lenz_lib::diagnose_windows_ocr(std::path::Path::new(&args[1])).unwrap_or(false);
            std::process::exit(if success { 0 } else { 1 });
        }
    }
    lenz_lib::run()
}
