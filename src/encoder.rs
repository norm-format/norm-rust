use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::error::NormError;
use crate::lexer::is_valid_section_name;

pub fn encode(value: &Value) -> Result<String, NormError> {
    match value {
        Value::Object(_) | Value::Array(_) => {}
        _ => return Err(NormError::ScalarRoot),
    }

    let mut planner = Planner::new();
    planner.plan_root(value)?;
    Ok(planner.render())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionKind {
    Table,
    Array,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Root,
    Object,
    ArrayOfObjects,
    PrimitiveArray,
}

#[derive(Debug)]
struct PlanSection {
    name: String,
    kind: SectionKind,
    role: Role,
    has_pk_column: bool,
    data_columns: Vec<String>,
    rows: Vec<PlanRow>,
    values: Vec<PlanCell>,
}

#[derive(Debug)]
struct PlanRow {
    pk: Option<u64>,
    fields: Vec<(String, PlanCell)>,
}

#[derive(Debug, Clone)]
enum PlanCell {
    Value(Value),
    RowRef(u64),
    NamedRef(String),
    EmptyArrayRef,
}

struct Planner {
    sections: Vec<PlanSection>,
    object_section_by_key: HashMap<String, usize>,
    used_names: HashMap<String, usize>,
    next_pk: u64,
    root_array: bool,
}

impl Planner {
    fn new() -> Self {
        Self {
            sections: Vec::new(),
            object_section_by_key: HashMap::new(),
            used_names: HashMap::new(),
            next_pk: 1,
            root_array: false,
        }
    }

    fn reserve_name(&mut self, base: &str) -> String {
        let sanitised = sanitise_name(base);
        if !self.used_names.contains_key(&sanitised) {
            self.used_names.insert(sanitised.clone(), 1);
            return sanitised;
        }
        let mut n = 2;
        loop {
            let candidate = format!("{}_{}", sanitised, n);
            if !self.used_names.contains_key(&candidate) {
                self.used_names.insert(candidate.clone(), 1);
                return candidate;
            }
            n += 1;
        }
    }

    fn plan_root(&mut self, value: &Value) -> Result<(), NormError> {
        let root_name = self.reserve_name("data");
        match value {
            Value::Object(obj) => {
                self.root_array = false;
                let mut section = PlanSection {
                    name: root_name,
                    kind: SectionKind::Table,
                    role: Role::Root,
                    has_pk_column: false,
                    data_columns: Vec::new(),
                    rows: Vec::new(),
                    values: Vec::new(),
                };
                let row = self.build_row(obj)?;
                merge_row_into_table(&mut section, row);
                self.sections.insert(0, section);
            }
            Value::Array(arr) => {
                self.root_array = true;
                if !arr.is_empty() && arr.iter().all(|v| matches!(v, Value::Object(_))) {
                    let mut section = PlanSection {
                        name: root_name,
                        kind: SectionKind::Table,
                        role: Role::ArrayOfObjects,
                        has_pk_column: false,
                        data_columns: Vec::new(),
                        rows: Vec::new(),
                        values: Vec::new(),
                    };
                    for item in arr {
                        if let Value::Object(obj) = item {
                            let row = self.build_row(obj)?;
                            merge_row_into_table(&mut section, row);
                        }
                    }
                    self.sections.insert(0, section);
                } else {
                    let base_name = root_name.clone();
                    let mut section = PlanSection {
                        name: root_name,
                        kind: SectionKind::Array,
                        role: Role::PrimitiveArray,
                        has_pk_column: false,
                        data_columns: Vec::new(),
                        rows: Vec::new(),
                        values: Vec::new(),
                    };
                    for item in arr {
                        let cell = self.build_array_item(item, &base_name)?;
                        section.values.push(cell);
                    }
                    self.sections.insert(0, section);
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn build_row(&mut self, obj: &Map<String, Value>) -> Result<PlanRow, NormError> {
        let mut fields = Vec::new();
        for (k, v) in obj {
            let cell = self.build_field_cell(v, k)?;
            fields.push((k.clone(), cell));
        }
        Ok(PlanRow { pk: None, fields })
    }

    fn build_field_cell(&mut self, v: &Value, key: &str) -> Result<PlanCell, NormError> {
        match v {
            Value::Object(obj) => {
                let pk = self.next_pk;
                self.next_pk += 1;
                let section_idx = self.get_or_create_object_section(key);
                let mut row = self.build_row(obj)?;
                row.pk = Some(pk);
                merge_row_into_table(&mut self.sections[section_idx], row);
                Ok(PlanCell::RowRef(pk))
            }
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Ok(PlanCell::EmptyArrayRef);
                }
                if arr.iter().all(|v| matches!(v, Value::Object(_))) {
                    let section_idx = self.create_array_of_objects_section(key);
                    for item in arr {
                        if let Value::Object(obj) = item {
                            let row = self.build_row(obj)?;
                            merge_row_into_table(&mut self.sections[section_idx], row);
                        }
                    }
                    let name = self.sections[section_idx].name.clone();
                    Ok(PlanCell::NamedRef(name))
                } else {
                    let name = self.reserve_name(key);
                    let section_idx = self.sections.len();
                    self.sections.push(PlanSection {
                        name: name.clone(),
                        kind: SectionKind::Array,
                        role: Role::PrimitiveArray,
                        has_pk_column: false,
                        data_columns: Vec::new(),
                        rows: Vec::new(),
                        values: Vec::new(),
                    });
                    for item in arr {
                        let cell = self.build_array_item(item, &name)?;
                        self.sections[section_idx].values.push(cell);
                    }
                    Ok(PlanCell::NamedRef(name))
                }
            }
            _ => Ok(PlanCell::Value(v.clone())),
        }
    }

    fn build_array_item(&mut self, v: &Value, parent_base: &str) -> Result<PlanCell, NormError> {
        match v {
            Value::Object(obj) => {
                let pk = self.next_pk;
                self.next_pk += 1;
                let section_key = format!("{}_item", parent_base);
                let section_idx = self.get_or_create_object_section(&section_key);
                let mut row = self.build_row(obj)?;
                row.pk = Some(pk);
                merge_row_into_table(&mut self.sections[section_idx], row);
                Ok(PlanCell::RowRef(pk))
            }
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Ok(PlanCell::EmptyArrayRef);
                }
                let child_base = format!("{}_r", parent_base);
                let name = self.reserve_name(&child_base);
                let section_idx = self.sections.len();
                let new_section = if arr.iter().all(|v| matches!(v, Value::Object(_))) {
                    PlanSection {
                        name: name.clone(),
                        kind: SectionKind::Table,
                        role: Role::ArrayOfObjects,
                        has_pk_column: false,
                        data_columns: Vec::new(),
                        rows: Vec::new(),
                        values: Vec::new(),
                    }
                } else {
                    PlanSection {
                        name: name.clone(),
                        kind: SectionKind::Array,
                        role: Role::PrimitiveArray,
                        has_pk_column: false,
                        data_columns: Vec::new(),
                        rows: Vec::new(),
                        values: Vec::new(),
                    }
                };
                self.sections.push(new_section);
                if self.sections[section_idx].kind == SectionKind::Table {
                    for item in arr {
                        if let Value::Object(obj) = item {
                            let row = self.build_row(obj)?;
                            merge_row_into_table(&mut self.sections[section_idx], row);
                        }
                    }
                } else {
                    for item in arr {
                        let cell = self.build_array_item(item, &name)?;
                        self.sections[section_idx].values.push(cell);
                    }
                }
                Ok(PlanCell::NamedRef(name))
            }
            _ => Ok(PlanCell::Value(v.clone())),
        }
    }

    fn get_or_create_object_section(&mut self, key: &str) -> usize {
        if let Some(&idx) = self.object_section_by_key.get(key) {
            if self.sections[idx].role == Role::Object {
                return idx;
            }
        }
        let name = self.reserve_name(key);
        let idx = self.sections.len();
        self.sections.push(PlanSection {
            name,
            kind: SectionKind::Table,
            role: Role::Object,
            has_pk_column: true,
            data_columns: Vec::new(),
            rows: Vec::new(),
            values: Vec::new(),
        });
        self.object_section_by_key.insert(key.to_string(), idx);
        idx
    }

    fn create_array_of_objects_section(&mut self, key: &str) -> usize {
        let name = self.reserve_name(key);
        let idx = self.sections.len();
        self.sections.push(PlanSection {
            name,
            kind: SectionKind::Table,
            role: Role::ArrayOfObjects,
            has_pk_column: false,
            data_columns: Vec::new(),
            rows: Vec::new(),
            values: Vec::new(),
        });
        idx
    }

    fn render(&self) -> String {
        let mut out = String::new();
        if self.root_array {
            out.push_str(":root[]\n");
        } else {
            out.push_str(":root\n");
        }
        for (i, section) in self.sections.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            render_section(&mut out, section);
        }
        out
    }
}

fn merge_row_into_table(section: &mut PlanSection, row: PlanRow) {
    for (k, _) in &row.fields {
        if !section.data_columns.iter().any(|c| c == k) {
            section.data_columns.push(k.clone());
        }
    }
    section.rows.push(row);
}

fn render_section(out: &mut String, section: &PlanSection) {
    out.push(':');
    out.push_str(&section.name);
    if section.kind == SectionKind::Array {
        out.push_str("[]");
    }
    out.push('\n');

    match section.kind {
        SectionKind::Table => {
            let mut parts: Vec<String> = Vec::new();
            if section.has_pk_column {
                parts.push("pk".to_string());
            }
            for col in &section.data_columns {
                parts.push(csv_field_header(col));
            }
            out.push_str(&parts.join(","));
            out.push('\n');

            for row in &section.rows {
                let mut cells: Vec<String> = Vec::new();
                if section.has_pk_column {
                    cells.push(row.pk.map(|n| n.to_string()).unwrap_or_default());
                }
                for col in &section.data_columns {
                    if let Some((_, cell)) = row.fields.iter().find(|(k, _)| k == col) {
                        cells.push(render_plan_cell(cell));
                    } else {
                        cells.push(String::new());
                    }
                }
                out.push_str(&cells.join(","));
                out.push('\n');
            }
        }
        SectionKind::Array => {
            for cell in &section.values {
                out.push_str(&render_plan_cell(cell));
                out.push('\n');
            }
        }
    }
}

fn render_plan_cell(cell: &PlanCell) -> String {
    match cell {
        PlanCell::Value(v) => render_scalar(v),
        PlanCell::RowRef(n) => format!("@{}", n),
        PlanCell::NamedRef(name) => format!("@{}", name),
        PlanCell::EmptyArrayRef => "@[]".to_string(),
    }
}

fn render_scalar(v: &Value) -> String {
    match v {
        Value::String(s) => {
            if needs_quoting(s) {
                quote_csv(s)
            } else {
                s.to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Null => "null".to_string(),
        Value::Array(_) | Value::Object(_) => String::new(),
    }
}

fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    let trimmed_left = s.trim_start_matches([' ', '\t']);
    let trimmed = trimmed_left.trim_end_matches([' ', '\t']);
    if trimmed != s {
        return true;
    }
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        return true;
    }
    if s == "true" || s == "false" || s == "null" {
        return true;
    }
    if s.starts_with('@') {
        return true;
    }
    if looks_like_number(s) {
        return true;
    }
    false
}

fn looks_like_number(s: &str) -> bool {
    matches!(serde_json::from_str::<Value>(s), Ok(Value::Number(_)))
}

fn quote_csv(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        if ch == '"' {
            out.push('"');
            out.push('"');
        } else {
            out.push(ch);
        }
    }
    out.push('"');
    out
}

fn csv_field_header(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        quote_csv(s)
    } else {
        s.to_string()
    }
}

fn sanitise_name(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        let valid = if i == 0 {
            ch.is_ascii_alphabetic() || ch == '_'
        } else {
            ch.is_ascii_alphanumeric() || ch == '_'
        };
        out.push(if valid { ch } else { '_' });
    }
    if out.is_empty() {
        out.push('_');
    }
    debug_assert!(is_valid_section_name(&out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_scalar_root() {
        assert_eq!(encode(&json!(42)), Err(NormError::ScalarRoot));
        assert_eq!(encode(&json!("hi")), Err(NormError::ScalarRoot));
        assert_eq!(encode(&json!(null)), Err(NormError::ScalarRoot));
        assert_eq!(encode(&json!(true)), Err(NormError::ScalarRoot));
    }

    #[test]
    fn encodes_empty_object() {
        let out = encode(&json!({})).unwrap();
        assert!(out.contains(":root"));
    }

    #[test]
    fn encodes_empty_array() {
        let out = encode(&json!([])).unwrap();
        assert!(out.contains(":root[]"));
    }

    #[test]
    fn needs_quoting_rules() {
        assert!(needs_quoting(""));
        assert!(needs_quoting("42"));
        assert!(needs_quoting("true"));
        assert!(needs_quoting("false"));
        assert!(needs_quoting("null"));
        assert!(needs_quoting("@foo"));
        assert!(needs_quoting("a,b"));
        assert!(needs_quoting("a\"b"));
        assert!(needs_quoting("a\nb"));
        assert!(needs_quoting("  padded  "));
        assert!(!needs_quoting("hello"));
        assert!(!needs_quoting("color #FF0000"));
    }

    #[test]
    fn sanitise_name_rules() {
        assert_eq!(sanitise_name("foo"), "foo");
        assert_eq!(sanitise_name("foo-bar"), "foo_bar");
        assert_eq!(sanitise_name("1foo"), "_foo");
        assert_eq!(sanitise_name(""), "_");
        assert_eq!(sanitise_name("my.key.name"), "my_key_name");
    }

    #[test]
    fn quote_csv_doubles_quotes() {
        assert_eq!(quote_csv("a\"b"), "\"a\"\"b\"");
        assert_eq!(quote_csv("plain"), "\"plain\"");
    }

    #[test]
    fn assigns_sequential_pks_to_nested_objects() {
        let v = json!({
            "a": {"x": 1},
            "b": {"x": 2},
            "c": {"x": 3}
        });
        let out = encode(&v).unwrap();
        assert!(out.contains("@1"), "expected @1 ref, got:\n{}", out);
        assert!(out.contains("@2"), "expected @2 ref, got:\n{}", out);
        assert!(out.contains("@3"), "expected @3 ref, got:\n{}", out);
        assert!(out.contains("1,1\n"), "expected pk=1 row, got:\n{}", out);
        assert!(out.contains("2,2\n"), "expected pk=2 row, got:\n{}", out);
        assert!(out.contains("3,3\n"), "expected pk=3 row, got:\n{}", out);
    }

    #[test]
    fn section_name_collision_suffixes() {
        let v = json!({"data": {"x": 1}});
        let out = encode(&v).unwrap();
        assert!(
            out.contains(":data\n"),
            "expected root :data section, got:\n{}",
            out
        );
        assert!(
            out.contains(":data_2\n"),
            "expected suffixed :data_2 section, got:\n{}",
            out
        );
    }

    #[test]
    fn repeated_collisions_increment_suffix() {
        let v = json!({
            "data": {"x": 1},
            "Data": {"y": 2}
        });
        let out = encode(&v).unwrap();
        assert!(out.contains(":data\n"), "got:\n{}", out);
        assert!(out.contains(":data_2\n"), "got:\n{}", out);
        assert!(out.contains(":Data\n"), "got:\n{}", out);
    }

    #[test]
    fn literal_pk_key_produces_double_pk_header() {
        let v = json!({"item": {"pk": "XYZ-001", "name": "Alice"}});
        let out = encode(&v).unwrap();
        assert!(
            out.contains("pk,pk,name\n"),
            "expected pk,pk,name header, got:\n{}",
            out
        );
        assert!(
            out.contains("1,XYZ-001,Alice\n"),
            "expected row with structural pk=1 followed by data pk, got:\n{}",
            out
        );
    }

    #[test]
    fn sanitises_invalid_key_characters_in_section_name() {
        let v = json!({"foo-bar": {"x": 1}});
        let out = encode(&v).unwrap();
        assert!(
            out.contains(":foo_bar\n"),
            "expected sanitised section name, got:\n{}",
            out
        );
    }
}
