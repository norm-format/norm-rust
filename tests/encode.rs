use serde_json::Value;

fn load_expected(name: &str) -> Value {
    let s = std::fs::read_to_string(format!("tests/fixtures/{}.json", name)).unwrap();
    serde_json::from_str(&s).unwrap()
}

fn encode_and_roundtrip(name: &str) {
    let expected = load_expected(name);
    let norm = norm_codec::encode(&expected).expect("encode");
    let actual = norm_codec::parse(&norm)
        .unwrap_or_else(|e| panic!("re-parse {} failed: {}\nNORM was:\n{}", name, e, norm));
    assert_eq!(actual, expected, "round-trip mismatch for {}", name);
}

#[test]
fn encode_object_root() {
    encode_and_roundtrip("object_root");
}

#[test]
fn encode_array_root() {
    encode_and_roundtrip("array_root");
}

#[test]
fn encode_references() {
    encode_and_roundtrip("references");
}

#[test]
fn encode_empty_structures() {
    encode_and_roundtrip("empty_structures");
}

#[test]
fn encode_heterogeneous() {
    encode_and_roundtrip("heterogeneous");
}

#[test]
fn encode_nested_arrays() {
    encode_and_roundtrip("nested_arrays");
}

#[test]
fn encode_quoting() {
    encode_and_roundtrip("quoting");
}

#[test]
fn encode_csv_escaping() {
    encode_and_roundtrip("csv_escaping");
}

#[test]
fn encode_pk_collision() {
    encode_and_roundtrip("pk_collision");
}

#[test]
fn encode_solar_system() {
    encode_and_roundtrip("solar_system");
}

#[test]
fn rejects_scalar_root_number() {
    let err = norm_codec::encode(&serde_json::json!(42)).unwrap_err();
    assert!(matches!(err, norm_codec::NormError::ScalarRoot));
}

#[test]
fn rejects_scalar_root_string() {
    let err = norm_codec::encode(&Value::String("hello".into())).unwrap_err();
    assert!(matches!(err, norm_codec::NormError::ScalarRoot));
}

#[test]
fn rejects_scalar_root_null() {
    let err = norm_codec::encode(&Value::Null).unwrap_err();
    assert!(matches!(err, norm_codec::NormError::ScalarRoot));
}

#[test]
fn rejects_scalar_root_bool() {
    let err = norm_codec::encode(&Value::Bool(true)).unwrap_err();
    assert!(matches!(err, norm_codec::NormError::ScalarRoot));
}
