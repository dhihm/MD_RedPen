use std::process::ExitCode;

use md_redpen::{cli::Cli, storage::DocumentSnapshot, terminal};
use thiserror::Error;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), MainError> {
    let cli = Cli::from_env();
    let snapshot = DocumentSnapshot::load(&cli.markdown)?;
    terminal::run(snapshot)?;
    Ok(())
}

#[derive(Debug, Error)]
enum MainError {
    #[error(transparent)]
    Storage(#[from] md_redpen::storage::StorageError),
    #[error(transparent)]
    Terminal(#[from] md_redpen::terminal::TuiError),
}
