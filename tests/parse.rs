use serde_json::Value;

fn load(name: &str) -> (String, Value) {
    let norm = std::fs::read_to_string(format!("tests/fixtures/{}.norm", name))
        .expect("read .norm fixture");
    let json_str = std::fs::read_to_string(format!("tests/fixtures/{}.json", name))
        .expect("read .json fixture");
    let expected: Value = serde_json::from_str(&json_str).expect("parse expected json");
    (norm, expected)
}

fn check(name: &str) {
    let (norm, expected) = load(name);
    let actual = norm_codec::parse(&norm).expect("parse should succeed");
    assert_eq!(actual, expected, "fixture {} mismatch", name);
}

#[test]
fn object_root() {
    check("object_root");
}

#[test]
fn array_root() {
    check("array_root");
}

#[test]
fn references() {
    check("references");
}

#[test]
fn empty_structures() {
    check("empty_structures");
}

#[test]
fn heterogeneous() {
    check("heterogeneous");
}

#[test]
fn nested_arrays() {
    check("nested_arrays");
}

#[test]
fn comments() {
    check("comments");
}

#[test]
fn quoting() {
    check("quoting");
}

#[test]
fn csv_escaping() {
    check("csv_escaping");
}

#[test]
fn pk_collision() {
    check("pk_collision");
}

#[test]
fn solar_system() {
    check("solar_system");
}
