//! Read-only developer harness until the desktop UI is implemented.

use std::env;
use std::path::Path;
use std::process::ExitCode;

use namifl_core::discovery::scan_folder;

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.is_empty() || (args.len() == 1 && (args[0] == "--help" || args[0] == "-h")) {
        println!(
            "NamiFL — Cull → DxO → Transport\n\nUsage: namifl inspect <folder>\n\nRead-only skeleton: lists photo groups; does not modify or transfer files."
        );
        return ExitCode::SUCCESS;
    }
    if args.len() != 2 || args[0] != "inspect" {
        eprintln!("Usage: namifl inspect <folder>");
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
