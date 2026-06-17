use std::process::ExitCode;

use open_razer_battery::{cli, error::AppError, tray};
#[cfg(all(windows, not(debug_assertions)))]
use windows::Win32::System::Console::FreeConsole;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error.user_message());
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), AppError> {
    match std::env::args().nth(1).as_deref() {
        None | Some("tray") => {
            detach_console_for_tray();
            tray::run()
        }
        Some("list") => cli::run_list(),
        Some("probe") => cli::run_probe(),
        Some("-h") | Some("--help") => {
            print_usage();
            Ok(())
        }
        Some(_) => {
            print_usage();
            Err(AppError::InvalidCommand)
        }
    }
}

fn print_usage() {
    eprintln!("Usage: razer-bat.exe [list|probe|tray]");
}

fn detach_console_for_tray() {
    #[cfg(all(windows, not(debug_assertions)))]
    unsafe {
        let _ = FreeConsole();
    }
}
