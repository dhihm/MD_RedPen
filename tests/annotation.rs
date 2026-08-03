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
        "1. **부연 설명**: 직접 쓴 설명\n\n",
        "[rp-7k3m]: #rp-note-7k3m\n",
        "<!-- md-redpen:notes:end -->\n",
    );

    assert_eq!(actual, expected);
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
