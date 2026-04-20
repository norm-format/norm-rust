use serde_json::Value;

fn load_norm(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{}.norm", name)).unwrap()
}

fn roundtrip(name: &str) {
    let norm = load_norm(name);
    let v1: Value = norm_codec::parse(&norm).expect("first parse");
    let re_encoded = norm_codec::encode(&v1).expect("encode");
    let v2: Value = norm_codec::parse(&re_encoded).unwrap_or_else(|e| {
        panic!(
            "second parse failed: {}\nRe-encoded NORM:\n{}",
            e, re_encoded
        )
    });
    assert_eq!(v1, v2, "round-trip drift for {}", name);
}

#[test]
fn rt_object_root() {
    roundtrip("object_root");
}

#[test]
fn rt_array_root() {
    roundtrip("array_root");
}

#[test]
fn rt_references() {
    roundtrip("references");
}

#[test]
fn rt_empty_structures() {
    roundtrip("empty_structures");
}

#[test]
fn rt_heterogeneous() {
    roundtrip("heterogeneous");
}

#[test]
fn rt_nested_arrays() {
    roundtrip("nested_arrays");
}

#[test]
fn rt_quoting() {
    roundtrip("quoting");
}

#[test]
fn rt_csv_escaping() {
    roundtrip("csv_escaping");
}

#[test]
fn rt_pk_collision() {
    roundtrip("pk_collision");
}

#[test]
fn rt_solar_system() {
    roundtrip("solar_system");
}
