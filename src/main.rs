use std::io::{ErrorKind, Write};
use std::process::ExitCode;

use clap::Parser;
use gcp_billing_top::cli::{Args, run};

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args, &mut std::io::stdin().lock()) {
        Ok(text) => match std::io::stdout().lock().write_all(text.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) if e.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
            Err(e) => {
                let _ = writeln!(std::io::stderr(), "error: cannot write output: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "error: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}
