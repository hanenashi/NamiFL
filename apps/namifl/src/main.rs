//! Desktop culling prototype and read-only inspection command.

mod desktop;
mod media;
mod session;
mod settings;

use std::env;
use std::path::Path;
use std::process::ExitCode;

use namifl_core::discovery::scan_folder;

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!(
            "NamiFL — Cull → DxO → Transport\n\nUsage:\n  namifl [cull [folder]]\n  namifl inspect <folder>\n\nThe culling prototype saves review decisions separately from source photos.\nSource files stay unchanged until you explicitly confirm Delete cleanup. Transport is not implemented.\nSet NAMIFL_STATE_DIR to use a separate review-session directory."
        );
        return ExitCode::SUCCESS;
    }
    if args.is_empty() || (args[0] == "cull" && args.len() <= 2) {
        let folder = args.get(1).map(std::path::PathBuf::from);
        let state_root = env::var_os("NAMIFL_STATE_DIR")
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::data_local_dir().map(|root| root.join("NamiFL").join("sessions")));
        let Some(state_root) = state_root else {
            eprintln!("No per-user data directory is available. Set NAMIFL_STATE_DIR.");
            return ExitCode::FAILURE;
        };
        return match desktop::run(folder, state_root) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Could not start NamiFL: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if args.len() != 2 || args[0] != "inspect" {
        eprintln!("Usage: namifl [cull [folder]] | namifl inspect <folder>");
        return ExitCode::from(2);
    }
    match scan_folder(Path::new(&args[1])) {
        Ok(report) => {
            for group in &report.groups {
                let note = if group.is_ambiguous() {
                    " [ambiguous]"
                } else {
                    ""
                };
                println!(
                    "{}: {} JPEG(s), {} RAW(s){}",
                    group.stem,
                    group.jpegs.len(),
                    group.raws.len(),
                    note
                );
            }
            println!(
                "{} group(s); {} unsupported filename(s)",
                report.groups.len(),
                report.unsupported_names.len()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Could not inspect folder: {error}");
            ExitCode::FAILURE
        }
    }
}
