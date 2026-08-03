#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use md_redpen::{
    app::App, app_state::Mode, codex::CodexClient, storage::DocumentSnapshot, theme, ui,
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::{Buffer, Cell},
    style::{Color, Modifier},
};

#[test]
fn errors_are_prefixed_and_bold() -> Result<(), Box<dyn std::error::Error>> {
    let (_directory, mut app) = app_for("본문\n")?;
    app.handle_key(key(KeyCode::Char('v')));
    app.handle_key(key(KeyCode::Char('a')));
    app.handle_key(key(KeyCode::Enter));

    let buffer = render(&app, 100, 28)?;
    let text = buffer_text(&buffer);

    assert!(text.contains("Error: Note cannot be empty"));
    assert!(
        buffer
            .content()
            .iter()
            .any(|cell| cell.fg == theme::ERROR_RED && cell.modifier.contains(Modifier::BOLD))
    );
    Ok(())
}

#[test]
fn empty_document_has_centered_message() -> Result<(), Box<dyn std::error::Error>> {
    let (_directory, app) = app_for("")?;

    let buffer = render(&app, 100, 28)?;
    let text = buffer_text(&buffer);
    let empty_cell = find_cell(&buffer, "E").ok_or("empty message must render")?;

    assert!(text.contains("Empty Markdown document"));
    assert!(empty_cell.0 > 20);
    Ok(())
}

#[test]
fn compact_input_uses_one_line_status() -> Result<(), Box<dyn std::error::Error>> {
    let (_directory, mut app) = app_for("짧은 문장.\n")?;
    app.handle_key(key(KeyCode::Char('v')));
    app.handle_key(key(KeyCode::Char('a')));

    let buffer = render(&app, 60, 16)?;
    let text = buffer_text(&buffer);
    let panel_rows: Vec<u16> = buffer
        .content()
        .iter()
        .enumerate()
        .filter(|(_, cell)| cell.symbol() == "┌")
        .map(|(index, _)| index as u16 / buffer.area().width)
        .collect();

    assert!(!text.contains("type note"));
    assert!(panel_rows.get(1).is_some_and(|row| *row <= 8));
    let status = row_text(&buffer, buffer.area().height - 1);
    assert!(status.contains("Enter save"));
    assert!(status.contains("Esc cancel"));
    Ok(())
}

#[test]
fn codex_running_renders_deterministic_spinner() -> Result<(), Box<dyn std::error::Error>> {
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
            "exec tail -f /dev/null\n",
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

    let first = buffer_text(&render(&app, 100, 28)?);
    app.tick();
    let second = buffer_text(&render(&app, 100, 28)?);

    assert!(first.contains('⠋'));
    assert!(second.contains('⠙'));
    app.shutdown()?;
    Ok(())
}

#[test]
fn endnote_label_and_destination_use_semantic_focus() -> Result<(), Box<dyn std::error::Error>> {
    let (_directory, mut app) = app_for("RDMA는 빠르다.\n")?;
    app.handle_key(key(KeyCode::Char('w')));
    app.handle_key(key(KeyCode::Char('a')));
    for character in "직접 설명".chars() {
        app.handle_key(key(KeyCode::Char(character)));
    }
    app.handle_key(key(KeyCode::Enter));

    let saved = render(&app, 100, 28)?;
    let (_, _, label) = find_cell(&saved, "사").ok_or("manual label must render")?;
    assert!(label.modifier.contains(Modifier::BOLD));
    let (_, _, note) = find_cell(&saved, "직").ok_or("manual note must render")?;
    if !app.no_color() {
        assert_eq!(label.fg, theme::MARKER_YELLOW);
        assert_eq!(label.bg, theme::NOTE_SURFACE);
        assert_eq!(note.bg, theme::NOTE_SURFACE);
    }
    let heading_row = find_row(&saved, "MD RedPen Notes").ok_or("notes heading must render")?;
    assert!(row_inner_text(&saved, heading_row - 1).trim().is_empty());

    app.handle_key(key(KeyCode::Enter));
    let focused = render(&app, 100, 28)?;
    let (_, _, destination) = find_cell(&focused, "사").ok_or("focused label must render")?;
    if !app.no_color() {
        assert_eq!(destination.fg, theme::FOCUS_BLUE);
    }
    assert!(destination.modifier.contains(Modifier::UNDERLINED));
    Ok(())
}

#[test]
fn active_selection_meets_normal_text_contrast() {
    assert!(
        contrast_ratio(theme::SELECTION_TEXT, theme::SELECTION_BLUE) >= 4.5,
        "active selection must meet WCAG AA normal-text contrast"
    );
}

fn app_for(source: &str) -> Result<(tempfile::TempDir, App), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let document = directory.path().join("document.md");
    fs::write(&document, source)?;
    let snapshot = DocumentSnapshot::load(&document)?;
    let client = CodexClient::at("/bin/false", directory.path());
    let app = App::with_codex(snapshot, client)?;
    Ok((directory, app))
}

fn render(app: &App, width: u16, height: u16) -> Result<Buffer, Box<dyn std::error::Error>> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| ui::render(frame, app))?;
    Ok(terminal.backend().buffer().clone())
}

fn buffer_text(buffer: &Buffer) -> String {
    buffer
        .content()
        .iter()
        .map(Cell::symbol)
        .collect::<Vec<_>>()
        .join("")
}

fn find_row(buffer: &Buffer, needle: &str) -> Option<u16> {
    (0..buffer.area().height).find(|row| row_text(buffer, *row).contains(needle))
}

fn row_text(buffer: &Buffer, row: u16) -> String {
    let width = buffer.area().width as usize;
    let start = row as usize * width;
    buffer.content()[start..start + width]
        .iter()
        .map(Cell::symbol)
        .collect::<Vec<_>>()
        .join("")
}

fn row_inner_text(buffer: &Buffer, row: u16) -> String {
    row_text(buffer, row)
        .chars()
        .skip(1)
        .take(usize::from(buffer.area().width.saturating_sub(2)))
        .collect()
}

fn find_cell<'a>(buffer: &'a Buffer, symbol: &str) -> Option<(u16, u16, &'a Cell)> {
    buffer
        .content()
        .iter()
        .enumerate()
        .find(|(_, cell)| cell.symbol() == symbol)
        .map(|(index, cell)| {
            let width = buffer.area().width;
            (index as u16 % width, index as u16 / width, cell)
        })
}

fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    let lighter = luminance(foreground).max(luminance(background));
    let darker = luminance(foreground).min(luminance(background));
    (lighter + 0.05) / (darker + 0.05)
}

fn luminance(color: Color) -> f64 {
    let Color::Rgb(red, green, blue) = color else {
        return 0.0;
    };
    [red, green, blue]
        .map(|channel| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        })
        .into_iter()
        .zip([0.2126, 0.7152, 0.0722])
        .map(|(channel, coefficient)| channel * coefficient)
        .sum()
}

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
