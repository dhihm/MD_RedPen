#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use md_redpen::{app::App, app_state::Mode, codex::CodexClient, storage::DocumentSnapshot};

#[test]
fn c_opens_choice_before_any_codex_request() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let document = directory.path().join("document.md");
    fs::write(&document, "느린 문장입니다.\n")?;
    let snapshot = DocumentSnapshot::load(&document)?;
    let client = CodexClient::at("/bin/false", directory.path());
    let mut app = App::with_codex(snapshot, client)?;

    select_word(&mut app);
    app.handle_key(key(KeyCode::Char('c')));

    assert_eq!(app.mode(), Mode::CodexChoice);
    assert_eq!(fs::read_to_string(&document)?, "느린 문장입니다.\n");

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.mode(), Mode::Visual);
    assert_eq!(app.editor().selected_text(), Some("느린"));

    app.handle_key(key(KeyCode::Char('c')));
    app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(app.mode(), Mode::RevisionInput);
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.mode(), Mode::RevisionInput);
    assert!(
        app.status()
            .contains("Revision instruction cannot be empty")
    );
    type_text(&mut app, "더 정확하게");
    assert_eq!(app.input(), "더 정확하게");

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.mode(), Mode::Visual);
    assert!(app.input().is_empty());
    assert_eq!(app.editor().selected_text(), Some("느린"));
    Ok(())
}

#[test]
fn revision_instruction_is_reviewed_before_replacing_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = revision_fixture()?;
    let mut app = fixture.app;

    start_revision(&mut app, "더 정확하게 바꿔 줘");

    assert_eq!(app.mode(), Mode::CodexRunning);
    assert_eq!(fs::read_to_string(&fixture.document)?, "느린 문장입니다.\n");

    app.wait_for_codex()?;

    assert_eq!(app.mode(), Mode::Review);
    assert_eq!(app.review(), "더 정확한");
    assert_eq!(fs::read_to_string(&fixture.document)?, "느린 문장입니다.\n");
    let prompt = fs::read_to_string(&fixture.stdin)?;
    assert!(prompt.contains("<revision_instruction>\n더 정확하게 바꿔 줘"));
    assert!(prompt.contains("<selection>\n느린\n</selection>"));

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.mode(), Mode::Browse, "status: {}", app.status());
    assert_eq!(
        fs::read_to_string(&fixture.document)?,
        "더 정확한 문장입니다.\n"
    );
    assert!(!fs::read_to_string(&fixture.document)?.contains("md-redpen:notes"));
    assert!(app.status().contains("문장 수정 적용"));
    Ok(())
}

#[test]
fn external_change_blocks_reviewed_revision() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = revision_fixture()?;
    let mut app = fixture.app;
    start_revision(&mut app, "더 정확하게 바꿔 줘");
    app.wait_for_codex()?;
    fs::write(&fixture.document, "외부 편집 내용\n")?;

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.mode(), Mode::Review);
    assert!(app.status().contains("document changed outside"));
    assert_eq!(fs::read_to_string(&fixture.document)?, "외부 편집 내용\n");
    Ok(())
}

struct RevisionFixture {
    _directory: tempfile::TempDir,
    document: PathBuf,
    stdin: PathBuf,
    app: App,
}

fn revision_fixture() -> Result<RevisionFixture, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let document = directory.path().join("document.md");
    let script = directory.path().join("codex");
    let stdin = directory.path().join("stdin.txt");
    fs::write(&document, "느린 문장입니다.\n")?;
    write_fake_codex(&script)?;
    let snapshot = DocumentSnapshot::load(&document)?;
    let client = CodexClient::at(&script, directory.path()).with_test_capture(
        directory.path().join("args.txt"),
        &stdin,
        directory.path().join("env.txt"),
    );
    let app = App::with_codex(snapshot, client)?;
    Ok(RevisionFixture {
        _directory: directory,
        document,
        stdin,
        app,
    })
}

fn write_fake_codex(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        path,
        concat!(
            "#!/bin/sh\n",
            "if [ \"$1\" = \"login\" ]; then\n",
            "  printf 'Logged in using ChatGPT\\n'\n",
            "  exit 0\n",
            "fi\n",
            "cat > \"$FAKE_CODEX_STDIN\"\n",
            "printf '더 정확한'\n",
        ),
    )?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn start_revision(app: &mut App, instruction: &str) {
    select_word(app);
    app.handle_key(key(KeyCode::Char('c')));
    app.handle_key(key(KeyCode::Char('r')));
    type_text(app, instruction);
    app.handle_key(key(KeyCode::Enter));
}

fn select_word(app: &mut App) {
    app.handle_key(key(KeyCode::Char('w')));
}

fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        app.handle_key(key(KeyCode::Char(character)));
    }
}

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
