use std::process::ExitCode;

use razer_bat::{cli, error::AppError, tray};

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
        None | Some("tray") => tray::run(),
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
