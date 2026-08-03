use md_redpen::markdown::project_text;

#[test]
fn maps_korean_graphemes_to_utf8_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let source = "한글 e\u{301}🙂";
    let projection = project_text(source)?;
    let actual: Vec<(&str, std::ops::Range<usize>)> = projection
        .graphemes()
        .iter()
        .map(|grapheme| (grapheme.text(), grapheme.source_range()))
        .collect();

    assert_eq!(
        actual,
        vec![
            ("한", 0..3),
            ("글", 3..6),
            (" ", 6..7),
            ("e\u{301}", 7..10),
            ("🙂", 10..14),
        ]
    );
    Ok(())
}
