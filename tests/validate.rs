use norm_codec::NormError;

fn count<F: Fn(&NormError) -> bool>(errors: &[NormError], pred: F) -> usize {
    errors.iter().filter(|e| pred(e)).count()
}

#[test]
fn collects_multiple_invalid_pks() {
    let input = ":root\n:data\nx\n@1\n\n:items\npk,name\n01,a\n02,b\n03,c\n";
    let errors = norm_codec::validate(input).unwrap_err();
    let invalid_pks = count(&errors, |e| matches!(e, NormError::InvalidPk { .. }));
    assert_eq!(
        invalid_pks, 3,
        "expected 3 InvalidPk errors, got {:?}",
        errors
    );
}

#[test]
fn collects_multiple_duplicate_pks() {
    let input = ":root\n:data\nx\n@1\n\n:items\npk,name\n1,a\n1,b\n1,c\n";
    let errors = norm_codec::validate(input).unwrap_err();
    let dup_pks = count(&errors, |e| matches!(e, NormError::DuplicatePk { .. }));
    assert_eq!(
        dup_pks, 2,
        "expected 2 DuplicatePk errors, got {:?}",
        errors
    );
}

#[test]
fn collects_multiple_unresolved_refs() {
    let input = ":root\n:data\na,b,c\n@missing1,@missing2,@missing3\n";
    let errors = norm_codec::validate(input).unwrap_err();
    let unresolved = count(&errors, |e| {
        matches!(e, NormError::UnresolvedReference { .. })
    });
    assert_eq!(
        unresolved, 3,
        "expected 3 UnresolvedReference errors, got {:?}",
        errors
    );
}

#[test]
fn collects_mixed_error_types() {
    let input =
        ":root\n:data\na,b\n@missing,@1\n\n:items\npk,name\n01,alpha\n02,beta\n\n:items\nk\nv\n";
    let errors = norm_codec::validate(input).unwrap_err();
    assert!(
        count(&errors, |e| matches!(e, NormError::InvalidPk { .. })) >= 2,
        "expected multiple InvalidPk, got {:?}",
        errors
    );
    assert!(
        count(&errors, |e| matches!(
            e,
            NormError::UnresolvedReference { .. }
        )) >= 1,
        "expected UnresolvedReference, got {:?}",
        errors
    );
    assert!(
        count(&errors, |e| matches!(
            e,
            NormError::DuplicateSectionName { .. }
        )) >= 1,
        "expected DuplicateSectionName, got {:?}",
        errors
    );
}

#[test]
fn collects_multiple_empty_array_rows() {
    let input = ":root[]\n:items[]\nalpha\n\nbeta\n\ngamma\n";
    let errors = norm_codec::validate(input).unwrap_err();
    let empty_rows = count(&errors, |e| matches!(e, NormError::EmptyArrayRow { .. }));
    assert!(
        empty_rows >= 2,
        "expected multiple EmptyArrayRow errors, got {:?}",
        errors
    );
}

#[test]
fn collects_unreachable_plus_resolution_errors() {
    let input = ":root\n:data\nx\n@missing\n\n:ghost1\nk\nv\n\n:ghost2\nk\nv\n";
    let errors = norm_codec::validate(input).unwrap_err();
    assert_eq!(
        count(&errors, |e| matches!(
            e,
            NormError::UnresolvedReference { .. }
        )),
        1
    );
    assert_eq!(
        count(&errors, |e| matches!(
            e,
            NormError::UnreachableSection { .. }
        )),
        2,
        "expected 2 UnreachableSection, got {:?}",
        errors
    );
}

#[test]
fn fatal_lex_error_short_circuits() {
    let input = "\u{feff}:root\n:data\nx\n@missing\n";
    let errors = norm_codec::validate(input).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], NormError::BomDetected));
}

#[test]
fn valid_document_returns_ok() {
    let input = ":root\n:data\nname,age\nAlice,30\n";
    assert!(norm_codec::validate(input).is_ok());
}

#[test]
fn parse_still_returns_first_error_only() {
    let input = ":root\n:data\nx\n@1\n\n:items\npk,name\n01,a\n02,b\n";
    let err = norm_codec::parse(input).unwrap_err();
    assert!(matches!(err, NormError::InvalidPk { .. }));
}
