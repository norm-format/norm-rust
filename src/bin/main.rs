use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use norm_codec::NormError;

#[derive(Parser)]
#[command(name = "norm", version, about = "NORM data format codec")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a NORM document and emit JSON
    Parse {
        /// Emit compact JSON (default: pretty-printed)
        #[arg(long)]
        compact: bool,
        /// Input file path (reads stdin if omitted)
        file: Option<PathBuf>,
    },
    /// Encode a JSON document as NORM
    Encode {
        /// Input file path (reads stdin if omitted)
        file: Option<PathBuf>,
    },
    /// Validate a NORM document; print all errors to stderr
    Validate {
        /// Input file path (reads stdin if omitted)
        file: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            let stderr = io::stderr();
            let _ = writeln!(stderr.lock(), "{}", msg);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Parse { compact, file } => {
            let input = read_input(file.as_deref())?;
            let value = norm_codec::parse(&input).map_err(|e: NormError| e.to_string())?;
            let stdout = io::stdout();
            let mut out = stdout.lock();
            let s = if compact {
                serde_json::to_string(&value)
            } else {
                serde_json::to_string_pretty(&value)
            }
            .map_err(|e| e.to_string())?;
            writeln!(out, "{}", s).map_err(|e| e.to_string())?;
            Ok(())
        }
        Command::Encode { file } => {
            let input = read_input(file.as_deref())?;
            let value: serde_json::Value =
                serde_json::from_str(&input).map_err(|e| e.to_string())?;
            let norm = norm_codec::encode(&value).map_err(|e: NormError| e.to_string())?;
            let stdout = io::stdout();
            let mut out = stdout.lock();
            out.write_all(norm.as_bytes()).map_err(|e| e.to_string())?;
            Ok(())
        }
        Command::Validate { file } => {
            let input = read_input(file.as_deref())?;
            match norm_codec::validate(&input) {
                Ok(()) => Ok(()),
                Err(errors) => {
                    let stderr = io::stderr();
                    let mut e = stderr.lock();
                    for err in &errors {
                        let _ = writeln!(e, "{}", err);
                    }
                    Err(format!("{} error(s)", errors.len()))
                }
            }
        }
    }
}

fn read_input(path: Option<&std::path::Path>) -> Result<String, String> {
    match path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("{}: {}", p.display(), e)),
        None => {
            let mut s = String::new();
            io::stdin()
                .read_to_string(&mut s)
                .map_err(|e| e.to_string())?;
            Ok(s)
        }
    }
}
