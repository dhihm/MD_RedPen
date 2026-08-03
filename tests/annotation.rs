use md_redpen::annotation::{AnnotationError, AnnotationId, AnnotationRequest, NoteKind, annotate};

#[test]
fn creates_clickable_mark_and_managed_endnote() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# 제목\n\n한글 문장입니다.\n";
    let start = source
        .find("한글 문장")
        .ok_or("fixture must contain selection")?;
    let end = start + "한글 문장".len();
    let request = AnnotationRequest {
        id: AnnotationId::parse("rp-7k3m")?,
        kind: NoteKind::Explanation,
        note: "직접 쓴 설명",
        selection: start..end,
    };

    let actual = annotate(source, &request)?;
    let expected = concat!(
        "# 제목\n\n",
        "<mark>[한글 문장][rp-7k3m]</mark>입니다.\n\n",
        "<!-- md-redpen:notes:start v=1 -->\n",
        "## MD RedPen Notes\n\n",
        "<a id=\"rp-note-7k3m\"></a>\n",
        "### 1) 한글 문장\n\n",
        "직접 쓴 설명\n\n",
        "[rp-7k3m]: #rp-note-7k3m\n",
        "<!-- md-redpen:notes:end -->\n",
    );

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn truncates_endnote_title_without_splitting_cjk() -> Result<(), Box<dyn std::error::Error>> {
    let selected = "가".repeat(30);
    let source = format!("{selected} 뒤 문장입니다.\n");
    let request = AnnotationRequest {
        id: AnnotationId::parse("rp-title")?,
        kind: NoteKind::Explanation,
        note: "설명",
        selection: 0..selected.len(),
    };

    let actual = annotate(&source, &request)?;
    let expected_title = format!("### 1) {}…", "가".repeat(23));

    assert!(actual.contains(&expected_title));
    assert!(actual.contains("\n\n설명\n\n"));
    assert!(!actual.contains("부연 설명"));
    Ok(())
}

#[test]
fn numbers_endnote_titles_in_document_order() -> Result<(), Box<dyn std::error::Error>> {
    let source = "첫 문장입니다.\n\n둘째 문장입니다.\n";
    let first_start = source.find("첫 문장").ok_or("missing first selection")?;
    let first_request = AnnotationRequest {
        id: AnnotationId::parse("rp-first")?,
        kind: NoteKind::Manual,
        note: "첫 설명",
        selection: first_start..first_start + "첫 문장".len(),
    };
    let first = annotate(source, &first_request)?;
    let second_start = first.find("둘째 문장").ok_or("missing second selection")?;
    let second_request = AnnotationRequest {
        id: AnnotationId::parse("rp-second")?,
        kind: NoteKind::Manual,
        note: "둘째 설명",
        selection: second_start..second_start + "둘째 문장".len(),
    };

    let actual = annotate(&first, &second_request)?;
    let first_title = actual.find("### 1) 첫 문장").ok_or("missing first title")?;
    let second_title = actual
        .find("### 2) 둘째 문장")
        .ok_or("missing second title")?;

    assert!(first_title < second_title);
    assert!(actual.contains("[rp-first]: #rp-note-first"));
    assert!(actual.contains("[rp-second]: #rp-note-second"));
    Ok(())
}

#[test]
fn rejects_selection_over_existing_link() -> Result<(), Box<dyn std::error::Error>> {
    let source = "Visit [example site](https://example.com).\n";
    let start = source
        .find("example site")
        .ok_or("fixture must contain link")?;
    let request = AnnotationRequest {
        id: AnnotationId::parse("rp-link")?,
        kind: NoteKind::Explanation,
        note: "설명",
        selection: start..start + "example site".len(),
    };

    let actual = annotate(source, &request);

    assert_eq!(actual, Err(AnnotationError::ExistingLinkOverlap));
    Ok(())
}

#[test]
fn rejects_overlapping_redpen_mark() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "<mark>[이미 강조][rp-old]</mark>된 문장.\n\n",
        "<!-- md-redpen:notes:start v=1 -->\n",
        "## MD RedPen Notes\n\n",
        "<a id=\"rp-note-old\"></a>\n",
        "1. **부연 설명**: 기존 설명\n\n",
        "[rp-old]: #rp-note-old\n",
        "<!-- md-redpen:notes:end -->\n",
    );
    let start = source
        .find("이미 강조")
        .ok_or("fixture must contain mark")?;
    let request = AnnotationRequest {
        id: AnnotationId::parse("rp-next")?,
        kind: NoteKind::Explanation,
        note: "새 설명",
        selection: start..start + "이미 강조".len(),
    };

    let actual = annotate(source, &request);

    assert_eq!(actual, Err(AnnotationError::ExistingAnnotationOverlap));
    Ok(())
}

#[test]
fn rejects_selection_inside_managed_notes() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = concat!(
        "본문입니다.\n\n",
        "<!-- md-redpen:notes:start v=1 -->\n",
        "## MD RedPen Notes\n\n",
        "기존 설명입니다.\n",
        "<!-- md-redpen:notes:end -->\n",
    );
    let start = source
        .find("기존 설명")
        .ok_or("fixture must contain managed note")?;
    let request = AnnotationRequest {
        id: AnnotationId::parse("rp-managed")?,
        kind: NoteKind::Explanation,
        note: "새 설명",
        selection: start..start + "기존 설명".len(),
    };

    // When
    let actual = annotate(source, &request);

    // Then
    assert_eq!(actual, Err(AnnotationError::ExistingAnnotationOverlap));
    Ok(())
}

#[test]
fn rejects_selection_over_inline_code() -> Result<(), Box<dyn std::error::Error>> {
    let source = "Use `cargo test` before release.\n";
    let start = source
        .find("cargo test")
        .ok_or("fixture must contain code")?;
    let request = AnnotationRequest {
        id: AnnotationId::parse("rp-code")?,
        kind: NoteKind::Explanation,
        note: "설명",
        selection: start..start + "cargo test".len(),
    };

    let actual = annotate(source, &request);

    assert_eq!(actual, Err(AnnotationError::InlineCodeOverlap));
    Ok(())
}

#[test]
fn rejects_selection_over_image() -> Result<(), Box<dyn std::error::Error>> {
    let source = "See ![diagram](architecture.png) here.\n";
    let start = source.find("diagram").ok_or("fixture must contain image")?;
    let request = AnnotationRequest {
        id: AnnotationId::parse("rp-image")?,
        kind: NoteKind::Explanation,
        note: "설명",
        selection: start..start + "diagram".len(),
    };

    let actual = annotate(source, &request);

    assert_eq!(actual, Err(AnnotationError::ImageOverlap));
    Ok(())
}
