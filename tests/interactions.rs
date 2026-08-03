use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use md_redpen::{app::App, app_state::Mode, codex::CodexClient, storage::DocumentSnapshot, ui};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Modifier};

#[test]
fn highlighted_sentence_navigates_to_its_endnote_and_back() -> Result<(), Box<dyn std::error::Error>>
{
    let source = concat!(
        "<mark>[선택 문장][rp-link]</mark>입니다.\n\n",
        "<!-- md-redpen:notes:start v=1 -->\n",
        "## MD RedPen Notes\n\n",
        "<a id=\"rp-note-link\"></a>\n",
        "### 1) 선택 문장\n\n",
        "설명입니다.\n\n",
        "[rp-link]: #rp-note-link\n",
        "<!-- md-redpen:notes:end -->\n",
    );
    let (_directory, mut app) = app_for(source)?;
    let body_cursor = app.editor().cursor();

    assert_eq!(app.editor().current_link_target(), Some("#rp-note-link"));
    app.handle_key(key(KeyCode::Enter));
    assert!(app.editor().cursor() > body_cursor);

    app.handle_key(key(KeyCode::Char('b')));
    assert_eq!(app.editor().cursor(), body_cursor);
    Ok(())
}

#[test]
fn arrow_down_moves_to_the_next_markdown_paragraph() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let document = directory.path().join("paragraphs.md");
    std::fs::write(
        &document,
        "첫 문단 첫 줄\n같은 문단 둘째 줄\n\n둘째 문단입니다.\n",
    )?;
    let snapshot = DocumentSnapshot::load(&document)?;
    let client = CodexClient::at("/bin/false", directory.path());
    let mut app = App::with_codex(snapshot, client)?;

    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Char('w')));

    assert_eq!(app.editor().selected_text(), Some("둘째"));
    Ok(())
}

#[test]
fn paragraph_navigation_keeps_the_cursor_in_view() -> Result<(), Box<dyn std::error::Error>> {
    let source = (1..=20)
        .map(|number| format!("문단 {number}\n\n"))
        .collect::<String>();
    let (_directory, mut app) = app_for(&source)?;

    for _ in 0..15 {
        app.handle_key(key(KeyCode::Down));
    }

    let backend = TestBackend::new(60, 16);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| ui::render(frame, &app))?;
    let buffer = terminal.backend().buffer();
    let width = usize::from(buffer.area().width);
    let cursor_cell =
        buffer.content().iter().enumerate().find(|(_, cell)| {
            cell.symbol() == "문" && cell.modifier.contains(Modifier::UNDERLINED)
        });
    let (cursor_offset, _) = cursor_cell.ok_or("paragraph cursor was not visible")?;
    let row_start = cursor_offset / width * width;
    let row = buffer.content()[row_start..row_start + width]
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();

    assert!(row.contains("16"));
    let column = u16::try_from(cursor_offset % width)?;
    let row = u16::try_from(cursor_offset / width)?;
    assert_eq!(
        ui::document_hit_test(&app, Rect::new(0, 0, 60, 16), column, row),
        Some(app.editor().cursor())
    );
    Ok(())
}

#[test]
fn mouse_drag_selects_exact_cjk_graphemes() -> Result<(), Box<dyn std::error::Error>> {
    let (_directory, mut app) = app_for("한글 문장과 RDMA는 빠르다.\n")?;
    let area = Rect::new(0, 0, 60, 16);
    let start = ui::document_hit_test(&app, area, 1, 1);
    let wide_cell = ui::document_hit_test(&app, area, 2, 1);
    let end = ui::document_hit_test(&app, area, 8, 1);

    assert_eq!(start, wide_cell);
    send_mouse(
        &mut app,
        area,
        MouseEventKind::Down(MouseButton::Left),
        start,
    );
    send_mouse(&mut app, area, MouseEventKind::Drag(MouseButton::Left), end);
    send_mouse(&mut app, area, MouseEventKind::Up(MouseButton::Left), end);

    assert_eq!(app.mode(), Mode::Visual);
    assert_eq!(app.editor().selected_text(), Some("한글 문장"));
    assert_eq!(app.status(), "마우스 선택됨 · a 수동 메모 · c Codex");
    let path = app.path().to_owned();
    app.handle_key(key(KeyCode::Char('a')));
    for character in "마우스로 선택".chars() {
        app.handle_key(key(KeyCode::Char(character)));
    }
    app.handle_key(key(KeyCode::Enter));
    let saved = std::fs::read_to_string(path)?;
    assert!(saved.contains("<mark>[한글 문장][rp-"));
    assert!(saved.contains("마우스로 선택"));
    Ok(())
}

#[test]
fn mouse_drag_keeps_scrolled_document_under_pointer() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "단락 01 첫 문장입니다.\n\n",
        "단락 02 둘째 문장입니다.\n\n",
        "단락 03 셋째 문장입니다.\n\n",
        "단락 04 넷째 문장입니다.\n\n",
        "단락 05 다섯째 문장입니다.\n\n",
        "단락 06 여섯째 문장입니다.\n\n",
        "단락 07 일곱째 문장입니다.\n\n",
        "단락 08 여덟째 문장입니다.\n\n",
        "단락 09 아홉째 문장입니다.\n\n",
        "단락 10 열째 문장입니다.\n\n",
        "단락 11 열한째 문장입니다.\n\n",
        "단락 12 열두째 문장입니다.\n\n",
        "단락 13 열셋째 문장입니다.\n\n",
        "단락 14 열넷째 문장입니다.\n\n",
        "단락 15 열다섯째 문장입니다.\n\n",
        "단락 16 열여섯째 문장입니다.\n\n",
        "단락 17 열일곱째 문장입니다.\n\n",
        "단락 18 열여덟째 문장입니다.\n",
    );
    let (_directory, mut app) = app_for(source)?;
    for _ in 1..18 {
        app.handle_key(key(KeyCode::Down));
    }
    let area = Rect::new(0, 0, 60, 16);
    let start = ui::document_hit_test(&app, area, 1, 1);

    send_mouse(
        &mut app,
        area,
        MouseEventKind::Down(MouseButton::Left),
        start,
    );

    assert_eq!(
        ui::document_hit_test(&app, area, 1, 1),
        start,
        "mouse down must not move the document under the pointer"
    );
    let end = ui::document_hit_test(&app, area, 7, 1);
    send_mouse(&mut app, area, MouseEventKind::Drag(MouseButton::Left), end);
    send_mouse(&mut app, area, MouseEventKind::Up(MouseButton::Left), end);
    assert_eq!(app.editor().selected_text(), Some("단락 06"));
    Ok(())
}

#[test]
fn mouse_hit_test_maps_wrapped_cjk_continuation_cells() -> Result<(), Box<dyn std::error::Error>> {
    let source = format!("{}\n", "가".repeat(30));
    let (_directory, app) = app_for(&source)?;
    let area = Rect::new(0, 0, 50, 12);
    let first_cell = ui::document_hit_test(&app, area, 1, 2);
    let second_cell = ui::document_hit_test(&app, area, 2, 2);

    assert_eq!(first_cell, Some(24));
    assert_eq!(second_cell, first_cell);
    Ok(())
}

#[test]
fn mouse_wheel_scrolls_document_without_moving_cursor() -> Result<(), Box<dyn std::error::Error>> {
    let source = (1..=18)
        .map(|number| format!("문단 {number:02} 내용입니다.\n\n"))
        .collect::<String>();
    let (_directory, mut app) = app_for(&source)?;
    let area = Rect::new(0, 0, 60, 16);
    let cursor = app.editor().cursor();
    let before = ui::document_hit_test(&app, area, 1, 1);

    send_mouse(&mut app, area, MouseEventKind::ScrollDown, None);

    let after = ui::document_hit_test(&app, area, 1, 1);
    assert_ne!(after, before);
    assert_eq!(app.editor().cursor(), cursor);

    send_mouse(&mut app, area, MouseEventKind::ScrollUp, None);
    assert_eq!(ui::document_hit_test(&app, area, 1, 1), before);
    Ok(())
}

#[test]
fn wide_boundary_help_keeps_mouse_and_quit_visible() -> Result<(), Box<dyn std::error::Error>> {
    let (_directory, app) = app_for("본문\n")?;
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| ui::render(frame, &app))?;
    let buffer = terminal.backend().buffer();
    let width = usize::from(buffer.area().width);
    let help = buffer.content()[23 * width..24 * width]
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();

    assert!(help.contains("drag select"));
    assert!(help.contains("wheel"));
    assert!(help.contains("q quit"));
    Ok(())
}

fn app_for(source: &str) -> Result<(tempfile::TempDir, App), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let document = directory.path().join("document.md");
    std::fs::write(&document, source)?;
    let snapshot = DocumentSnapshot::load(&document)?;
    let client = CodexClient::at("/bin/false", directory.path());
    let app = App::with_codex(snapshot, client)?;
    Ok((directory, app))
}

fn send_mouse(app: &mut App, area: Rect, kind: MouseEventKind, rendered_index: Option<usize>) {
    let (scroll_offset, max_scroll_offset) = ui::document_scroll_bounds(app, area);
    app.handle_mouse(kind, rendered_index, scroll_offset, max_scroll_offset);
}

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
