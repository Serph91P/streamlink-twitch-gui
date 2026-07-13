use std::{env, path::PathBuf, process::ExitCode};

use streamlink_twitch_gui_lib::updater_signature::verify_updater_signature;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let Some(artifact) = arguments.next().map(PathBuf::from) else {
        eprintln!("usage: verify-updater-signature ARTIFACT SIGNATURE");
        return ExitCode::FAILURE;
    };
    let Some(signature) = arguments.next().map(PathBuf::from) else {
        eprintln!("usage: verify-updater-signature ARTIFACT SIGNATURE");
        return ExitCode::FAILURE;
    };
    if arguments.next().is_some() {
        eprintln!("usage: verify-updater-signature ARTIFACT SIGNATURE");
        return ExitCode::FAILURE;
    }
    let public_key = match env::var("TAURI_UPDATER_PUBLIC_KEY") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("TAURI_UPDATER_PUBLIC_KEY is missing");
            return ExitCode::FAILURE;
        }
    };
    match verify_updater_signature(&public_key, &artifact, &signature) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
