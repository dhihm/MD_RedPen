use std::fs;

use md_redpen::storage::{DocumentSnapshot, StorageError, commit};

#[test]
fn external_change_aborts_atomic_commit() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("document.md");
    fs::write(&path, "원본\n")?;
    let snapshot = DocumentSnapshot::load(&path)?;
    fs::write(&path, "외부 변경\n")?;

    let actual = commit(&snapshot, "에이전트 변경\n");
    let persisted = fs::read_to_string(&path)?;

    assert_eq!(actual, Err(StorageError::ExternalChange));
    assert_eq!(persisted, "외부 변경\n");
    Ok(())
}
