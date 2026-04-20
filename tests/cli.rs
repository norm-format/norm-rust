#![cfg(feature = "cli")]

use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_norm"))
}

struct Output {
    status: i32,
    stdout: String,
    stderr: String,
}

fn run(mut cmd: Command, stdin: Option<&str>) -> Output {
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn norm binary");
    if let Some(s) = stdin {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(s.as_bytes())
            .unwrap();
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait on norm binary");
    Output {
        status: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8(out.stdout).expect("stdout utf8"),
        stderr: String::from_utf8(out.stderr).expect("stderr utf8"),
    }
}

fn fixture_path(name: &str) -> String {
    format!("tests/fixtures/{}.norm", name)
}

#[test]
fn parse_stdin_emits_pretty_json_exit_zero() {
    let input = ":root\n:data\nname,age\nAlice,30\n";
    let mut cmd = bin();
    cmd.arg("parse");
    let out = run(cmd, Some(input));
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert!(out.stderr.is_empty(), "unexpected stderr: {}", out.stderr);
    assert!(
        out.stdout.contains("\"name\": \"Alice\""),
        "stdout: {}",
        out.stdout
    );
    assert!(out.stdout.contains('\n'), "expected pretty-printed JSON");
}

#[test]
fn parse_compact_flag_emits_single_line_json() {
    let input = ":root\n:data\nname,age\nAlice,30\n";
    let mut cmd = bin();
    cmd.args(["parse", "--compact"]);
    let out = run(cmd, Some(input));
    assert_eq!(out.status, 0);
    let trimmed = out.stdout.trim_end_matches('\n');
    assert!(
        !trimmed.contains('\n'),
        "compact output must be single-line, got: {:?}",
        trimmed
    );
    assert_eq!(trimmed, r#"{"name":"Alice","age":30}"#);
}

#[test]
fn parse_file_argument_reads_from_disk() {
    let mut cmd = bin();
    cmd.args(["parse", "--compact", &fixture_path("object_root")]);
    let out = run(cmd, None);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert!(!out.stdout.is_empty());
}

#[test]
fn parse_invalid_input_exits_one_with_stderr() {
    let mut cmd = bin();
    cmd.arg("parse");
    let out = run(cmd, Some(":data\nk\nv\n"));
    assert_eq!(out.status, 1);
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);
    assert!(
        out.stderr.to_lowercase().contains("root"),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn parse_missing_file_exits_one() {
    let mut cmd = bin();
    cmd.args(["parse", "tests/fixtures/does_not_exist.norm"]);
    let out = run(cmd, None);
    assert_eq!(out.status, 1);
    assert!(!out.stderr.is_empty());
}

#[test]
fn encode_stdin_json_to_norm() {
    let input = r#"{"name":"Alice","age":30}"#;
    let mut cmd = bin();
    cmd.arg("encode");
    let out = run(cmd, Some(input));
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert!(out.stdout.starts_with(":root\n"), "stdout: {}", out.stdout);
    assert!(out.stdout.contains("name,age"), "stdout: {}", out.stdout);
    assert!(out.stdout.contains("Alice,30"), "stdout: {}", out.stdout);
}

#[test]
fn encode_invalid_json_exits_one() {
    let mut cmd = bin();
    cmd.arg("encode");
    let out = run(cmd, Some("not json"));
    assert_eq!(out.status, 1);
    assert!(!out.stderr.is_empty());
}

#[test]
fn validate_valid_input_exits_zero_silent() {
    let input = ":root\n:data\nname\nAlice\n";
    let mut cmd = bin();
    cmd.arg("validate");
    let out = run(cmd, Some(input));
    assert_eq!(out.status, 0);
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);
    assert!(out.stderr.is_empty(), "stderr: {}", out.stderr);
}

#[test]
fn validate_multi_error_input_reports_all_to_stderr() {
    let input = ":root\n:data\nx\n@1\n\n:items\npk,name\n01,a\n02,b\n03,c\n";
    let mut cmd = bin();
    cmd.arg("validate");
    let out = run(cmd, Some(input));
    assert_eq!(out.status, 1);
    let invalid_pk_lines = out
        .stderr
        .lines()
        .filter(|l| l.contains("invalid pk"))
        .count();
    assert_eq!(
        invalid_pk_lines, 3,
        "expected 3 invalid pk lines, stderr:\n{}",
        out.stderr
    );
}
