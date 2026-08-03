use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use md_redpen::{app::App, codex::CodexClient, storage::DocumentSnapshot, ui};
use ratatui::{Terminal, backend::TestBackend, buffer::Cell};

#[test]
fn codex_choice_and_revision_instruction_are_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let document = directory.path().join("document.md");
    std::fs::write(&document, "수정할 문장입니다.\n")?;
    let snapshot = DocumentSnapshot::load(&document)?;
    let client = CodexClient::at("/bin/false", directory.path());
    let mut app = App::with_codex(snapshot, client)?;

    app.handle_key(key(KeyCode::Char('w')));
    app.handle_key(key(KeyCode::Char('c')));
    let choice = render_text(&app)?;
    assert!(choice.contains("Codex action"));
    assert!(choice.contains("r  Revise sentence"));
    assert!(choice.contains("e  Generate automatic endnote"));

    app.handle_key(key(KeyCode::Char('r')));
    for character in "더 자연스럽게".chars() {
        app.handle_key(key(KeyCode::Char(character)));
    }
    let instruction = render_text(&app)?;
    assert!(instruction.contains("Revision instruction"));
    assert!(
        "더자연스럽게"
            .chars()
            .all(|character| instruction.contains(character))
    );
    assert!(instruction.contains("Enter send"));
    assert!(instruction.contains("Esc cancel"));
    Ok(())
}

fn render_text(app: &App) -> Result<String, Box<dyn std::error::Error>> {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| ui::render(frame, app))?;
    Ok(terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(Cell::symbol)
        .collect::<Vec<_>>()
        .join(""))
}

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
