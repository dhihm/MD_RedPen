#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use md_redpen::{app::App, app_state::Mode, codex::CodexClient, storage::DocumentSnapshot};

#[test]
fn codex_result_requires_review_before_commit() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let document = directory.path().join("document.md");
    let script = directory.path().join("codex");
    fs::write(&document, "RDMA는 빠르다.\n")?;
    fs::write(
        &script,
        concat!(
            "#!/bin/sh\n",
            "if [ \"$1\" = \"login\" ]; then\n",
            "  printf 'Logged in using ChatGPT\\n'\n",
            "  exit 0\n",
            "fi\n",
            "cat > /dev/null\n",
            "printf '가짜 Codex 설명'\n",
        ),
    )?;
    let mut permissions = fs::metadata(&script)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions)?;

    let snapshot = DocumentSnapshot::load(&document)?;
    let client = CodexClient::at(&script, directory.path());
    let mut app = App::with_codex(snapshot, client)?;

    app.handle_key(key(KeyCode::Char('w')));
    app.handle_key(key(KeyCode::Char('c')));

    assert_eq!(app.mode(), Mode::CodexRunning);
    assert_eq!(fs::read_to_string(&document)?, "RDMA는 빠르다.\n");

    app.wait_for_codex()?;

    assert_eq!(app.mode(), Mode::Review);
    assert_eq!(app.review(), "가짜 Codex 설명");
    assert_eq!(fs::read_to_string(&document)?, "RDMA는 빠르다.\n");

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.mode(), Mode::Browse);
    let persisted = fs::read_to_string(&document)?;
    assert!(persisted.contains("<mark>[RDMA]"));
    assert!(persisted.contains("가짜 Codex 설명"));
    Ok(())
}

#[test]
fn codex_draft_can_commit_prose_after_fenced_managed_marker_example()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let document = directory.path().join("document.md");
    let script = directory.path().join("codex");
    let source = concat!(
        "```markdown\n",
        "<!-- md-redpen:notes:start v=1 -->\n",
        "예시 미주\n",
        "<!-- md-redpen:notes:end -->\n",
        "```\n\n",
        "후속 문장입니다.\n",
    );
    fs::write(&document, source)?;
    fs::write(
        &script,
        concat!(
            "#!/bin/sh\n",
            "if [ \"$1\" = \"login\" ]; then\n",
            "  printf 'Logged in using ChatGPT\\n'\n",
            "  exit 0\n",
            "fi\n",
            "cat > /dev/null\n",
            "printf '가짜 Codex 설명'\n",
        ),
    )?;
    let mut permissions = fs::metadata(&script)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions)?;
    let snapshot = DocumentSnapshot::load(&document)?;
    let client = CodexClient::at(&script, directory.path());
    let mut app = App::with_codex(snapshot, client)?;
    let selection_start = source
        .find("후속 문장")
        .ok_or("fixture must contain selection")?;
    let target = app
        .editor()
        .projection()
        .graphemes()
        .iter()
        .position(|item| item.source_range().start == selection_start)
        .ok_or("selection must be rendered")?;
    while app.editor().cursor() < target {
        app.handle_key(key(KeyCode::Right));
    }
    app.handle_key(key(KeyCode::Char('w')));
    app.handle_key(key(KeyCode::Char('c')));
    app.wait_for_codex()?;
    assert_eq!(app.mode(), Mode::Review);

    // When
    app.handle_key(key(KeyCode::Enter));

    // Then
    assert_eq!(app.mode(), Mode::Browse, "status: {}", app.status());
    let persisted = fs::read_to_string(&document)?;
    let fenced_example = concat!(
        "```markdown\n",
        "<!-- md-redpen:notes:start v=1 -->\n",
        "예시 미주\n",
        "<!-- md-redpen:notes:end -->\n",
        "```\n\n",
    );
    assert!(persisted.starts_with(fenced_example));
    assert_eq!(
        persisted
            .matches("<!-- md-redpen:notes:start v=1 -->")
            .count(),
        2
    );
    assert_eq!(persisted.matches("<!-- md-redpen:notes:end -->").count(), 2);
    assert!(persisted.contains("<mark>[후속][rp-"));
    assert!(persisted.contains("가짜 Codex 설명"));
    Ok(())
}

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
