use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_describes_selection_keys() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_md-redpen"));

    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: md-redpen <MARKDOWN>"))
        .stdout(predicate::str::contains("v  start visual selection"))
        .stdout(predicate::str::contains(
            "a  add manual highlighted endnote",
        ))
        .stdout(predicate::str::contains("mouse drag  select rendered text"));
}

#[test]
fn missing_markdown_path_is_typed_error() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let missing = directory.path().join("missing.md");
    let mut command = Command::new(env!("CARGO_BIN_EXE_md-redpen"));

    command
        .arg(&missing)
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot read"))
        .stderr(predicate::str::contains("missing.md"));
    Ok(())
}
