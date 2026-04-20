use norm_codec::NormError;

fn load(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/errors/{}.norm", name))
        .expect("read error fixture")
}

#[test]
fn bom_detected() {
    let input = "\u{feff}:root\n:data\nk\nv\n";
    let err = norm_codec::parse(input).unwrap_err();
    assert!(matches!(err, NormError::BomDetected), "got {:?}", err);
}

#[test]
fn null_byte_detected() {
    let input = ":root\n:data\nk\nv\n\0\n";
    let err = norm_codec::parse(input).unwrap_err();
    assert!(matches!(err, NormError::NullByte { .. }), "got {:?}", err);
}

#[test]
fn missing_root_declaration() {
    let input = load("missing_root");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::MissingRootDeclaration),
        "got {:?}",
        err
    );
}

#[test]
fn invalid_root_declaration() {
    let input = ":root\n:data\nk\nv\n\n:root\nk\nv\n";
    let err = norm_codec::parse(input).unwrap_err();
    assert!(
        matches!(err, NormError::InvalidRootDeclaration { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn invalid_section_name() {
    let input = load("invalid_section_name");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::InvalidSectionName { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn duplicate_section_name() {
    let input = load("duplicate_section_name");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::DuplicateSectionName { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn unreachable_section() {
    let input = load("unreachable_section");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::UnreachableSection { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn duplicate_pk() {
    let input = load("duplicate_pk");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::DuplicatePk { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn invalid_pk_leading_zero() {
    let input = load("invalid_pk_leading_zero");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(matches!(err, NormError::InvalidPk { .. }), "got {:?}", err);
}

#[test]
fn unresolved_row_reference() {
    let input = load("unresolved_row_ref");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::UnresolvedReference { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn unresolved_named_reference() {
    let input = load("unresolved_named_ref");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::UnresolvedReference { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn circular_reference() {
    let input = load("circular_reference");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::CircularReference { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn multiple_root_rows() {
    let input = load("multiple_root_rows");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::MultipleRootRows { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn array_section_as_root() {
    let input = load("array_section_as_root");
    let err = norm_codec::parse(&input).unwrap_err();
    assert!(
        matches!(err, NormError::ArraySectionAsRoot { .. }),
        "got {:?}",
        err
    );
}

#[test]
fn empty_array_row() {
    let input = load("empty_array_row");
    let err = norm_codec::parse(&input).unwrap_err();
    match err {
        NormError::EmptyArrayRow { line } => {
            assert_eq!(line, 4, "expected error to point at the blank line");
        }
        other => panic!("got {:?}", other),
    }
}

#[test]
fn unexpected_data_row() {
    let input = load("unexpected_data_row");
    let err = norm_codec::parse(&input).unwrap_err();
    match err {
        NormError::UnexpectedDataRow { line } => {
            assert_eq!(line, 6, "expected error to point at the stray data row");
        }
        other => panic!("got {:?}", other),
    }
}

#[test]
fn encoder_rejects_scalar_root() {
    let err = norm_codec::encode(&serde_json::json!(42)).unwrap_err();
    assert!(matches!(err, NormError::ScalarRoot), "got {:?}", err);
}
