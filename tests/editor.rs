use md_redpen::editor::Editor;

#[test]
fn visual_selection_tracks_rendered_korean_graphemes() -> Result<(), Box<dyn std::error::Error>> {
    let mut editor = Editor::new("한글 문장을 선택합니다.\n")?;

    editor.start_visual();
    for _ in 0..4 {
        editor.move_right();
    }

    assert_eq!(editor.selected_text(), Some("한글 문장"));
    Ok(())
}

#[test]
fn word_selection_stops_before_korean_particle() -> Result<(), Box<dyn std::error::Error>> {
    let mut editor = Editor::new("RDMA는 빠르다.\n")?;

    editor.select_current_word();

    assert_eq!(editor.selected_text(), Some("RDMA"));
    Ok(())
}

#[test]
fn paragraph_navigation_skips_soft_breaks() -> Result<(), Box<dyn std::error::Error>> {
    let mut editor =
        Editor::new("첫 문단 첫 줄\n같은 문단 둘째 줄\n\n둘째 문단입니다.\n\n셋째 문단입니다.\n")?;

    editor.move_paragraph_down();
    editor.select_current_word();
    assert_eq!(editor.selected_text(), Some("둘째"));

    editor.clear_visual();
    editor.move_paragraph_down();
    editor.select_current_word();
    assert_eq!(editor.selected_text(), Some("셋째"));

    editor.clear_visual();
    editor.move_paragraph_up();
    editor.select_current_word();
    assert_eq!(editor.selected_text(), Some("둘째"));
    Ok(())
}
