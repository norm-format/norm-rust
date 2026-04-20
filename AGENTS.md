# norm-codec (Rust)

Rust library and optional CLI that converts between NORM (Normalised Object Relational Model) text and JSON. Tracks NORM spec v0.1. See `README.md` for public-facing overview and installation; this file covers what agents need to work in the source tree.

## Project Overview

- Crate name: `norm-codec` (library)
- Binary name: `norm` (feature-gated behind `cli`)
- Edition: 2021, license MPL-2.0
- Library surface: `parse`, `encode`, `validate`, `NormError`
- No `unsafe`, no file I/O in the library; callers own reading and writing
- Numeric precision and key order preserved via `serde_json` features `preserve_order` + `arbitrary_precision`

## Related Repositories

| Path | Purpose |
|------|---------|
| `../norm-spec/spec.md` | Authoritative prose specification (source of truth for MUST / MUST NOT rules) |
| `../norm-spec/abnf.md` | Draft grammar (prose wins on conflicts) |
| `../norm-spec/fixtures/` | Shared example inputs |

Consult `../norm-spec/spec.md` before changing parser, encoder, or error semantics. When prose and code disagree, the spec is correct and the code is wrong.

## Technology Stack

| Dependency | Purpose |
|------------|---------|
| `serde_json` 1 (`preserve_order`, `arbitrary_precision`) | JSON model; preserves key order and raw number tokens |
| `thiserror` 2 | `NormError` enum derive |
| `clap` 4 (`derive`, optional) | CLI argument parsing behind the `cli` feature |

Do not add dependencies without a clear need. The library must stay pure Rust, no `unsafe`, no I/O.

## Source Layout

```
src/
  lib.rs       public re-exports only
  error.rs     NormError enum (all public errors live here)
  lexer.rs     byte-level line tokeniser, BOM and null-byte checks, CSV cell parsing
  document.rs  internal Document / Section / Row types (pub(crate))
  parser.rs    NORM -> serde_json::Value; also hosts validate() aggregation
  encoder.rs   serde_json::Value -> NORM text
  bin/main.rs  CLI (compiled only with --features cli)
tests/
  parse.rs, encode.rs, roundtrip.rs, errors.rs, validate.rs, cli.rs
  fixtures/            happy-path .norm / .json pairs
  fixtures/errors/     one .norm per MUST-reject rule
```

Keep module boundaries: lexer emits tokens, parser builds `Document` then resolves references into JSON, encoder is the inverse. Do not cross-call between lexer and encoder.

## Public API

| Function | Signature | Semantics |
|----------|-----------|-----------|
| `parse` | `fn parse(input: &str) -> Result<Value, NormError>` | First error wins; returns owned `serde_json::Value` |
| `encode` | `fn encode(value: &Value) -> Result<String, NormError>` | First error wins; rejects scalar roots |
| `validate` | `fn validate(input: &str) -> Result<(), Vec<NormError>>` | Aggregates every error; fatal lex errors (BOM, null byte) short-circuit |

Only `parse`, `encode`, `validate`, and `NormError` are `pub`. Everything else is `pub(crate)`; do not widen visibility without a reason.

## Build, Test, Lint

```sh
cargo build                    # library only
cargo build --features cli     # library + norm binary
cargo test                     # runs every integration test file
cargo test --features cli      # required to include tests/cli.rs
cargo test --test parse        # run one integration test file
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all
cargo run --features cli -- parse tests/fixtures/solar_system.norm
```

IMPORTANT: `tests/cli.rs` is gated by `#![cfg(feature = "cli")]`; without `--features cli` it is silently skipped. Run with the feature before declaring CLI work complete.

NOTE: `arbitrary_precision` means JSON numbers round-trip as their original lexical form. Do not parse numeric cells to `f64` / `i64` inside the codec — route through `serde_json::Number` via its raw representation.

## Testing Conventions

- Integration tests live in `tests/` and must not import `pub(crate)` items
- Happy-path fixtures are `.norm` + `.json` pairs with matching stems under `tests/fixtures/`
- Roundtrip tests parse the `.norm`, compare to the `.json`, re-encode, and re-parse to the same JSON
- Error-rejection rules use one-test-per-rule in `tests/errors.rs`, each paired with one `.norm` file under `tests/fixtures/errors/`
- `tests/validate.rs` covers multi-error aggregation; add a case there when adding a new error that can occur more than once in a document
- CLI behaviour (exit codes, stdin/stdout/stderr split, `--compact`) is covered in `tests/cli.rs`

When adding a new MUST-reject rule:

1. Add a variant to `NormError` in `src/error.rs`
2. Emit it from the relevant stage (lexer, parser, encoder)
3. Create `tests/fixtures/errors/<rule>.norm` with the minimum reproducer
4. Add a single-rule test to `tests/errors.rs` matching on the variant
5. If the rule can fire multiple times, add an aggregation test to `tests/validate.rs`

When adding a new happy-path fixture, add both `foo.norm` and `foo.json` and wire it into `tests/parse.rs`, `tests/encode.rs`, and `tests/roundtrip.rs` as applicable.

## NORM Format Gotchas

These are the implementation pitfalls that keep recurring. Read `../norm-spec/spec.md` for the authoritative rules.

- UTF-8 BOM and embedded null bytes are fatal lex errors; they short-circuit `validate`
- The `pk` column is structural and MUST be stripped from the reconstructed JSON
- `pk` values are decimal digits with no leading zeros; `01` is an error, not the integer 1
- `pk` uniqueness is global across every table section, not per-section
- `@N` resolves to the row with `pk=N` and reconstructs as an object; `@name` resolves to a whole section (array); `@[]` is the literal empty array
- Empty unquoted cell means absent key, not JSON `null`; `null` as a bare token is JSON null
- Comments: `#` on a line by itself or after a structural line (`:root`, `:section`) is a comment. `#` inside a data row is literal text. Do not strip `#` from data rows.
- Quoting follows CSV rules: `"..."` with `""` as an escaped quote. Unquoted cells are taken literally (including whitespace at edges, per spec)
- Section names match `[a-zA-Z_][a-zA-Z0-9_]*`; purely numeric names are not reachable via `@name`
- The first section after the root declaration is the root content by position, not by name. `:root` means object root (one row); `:root[]` means array root
- Unreachable sections (defined but not reached from the root) are an error; the reachability walk starts at the root section and follows `@N` / `@name` references

## Error Reporting Conventions

- Every error variant that can be tied to a location carries a `line: usize` (1-based)
- `UnreachableSection` and `CircularReference` carry names instead of lines because the condition is graph-level
- `ScalarRoot` is encoder-only
- Error messages are lowercase, no trailing period, no ANSI. The CLI prints them verbatim to stderr.
- Do not add `String` fields to error variants beyond what is already there; keep variants cheap to construct and match on

## CLI Behaviour

| Command | stdin if no file | Success exit | Failure exit |
|---------|------------------|--------------|--------------|
| `norm parse [--compact] [FILE]` | yes | 0, JSON on stdout | 1, message on stderr |
| `norm encode [FILE]` | yes | 0, NORM on stdout (no trailing newline added) | 1, message on stderr |
| `norm validate [FILE]` | yes | 0, silent | 1, every error on stderr plus trailing `N error(s)` |

`parse` pretty-prints by default; `--compact` emits a single line with a trailing newline. Missing input files exit 1 with the OS error on stderr. These contracts are covered in `tests/cli.rs` — if you change CLI output, update the tests in the same change.

## Code Style

- Run `cargo fmt` before committing; CI assumes default rustfmt settings (no custom `rustfmt.toml`)
- `cargo clippy --all-targets --all-features -- -D warnings` must pass
- Prefer `&str` and `&[u8]` slices in the lexer; avoid allocating per line
- Do not add `unsafe`
- Keep public API surface minimal. New crate-public helpers go behind `pub(crate)`
- Do not introduce panics on invalid input. Every user-visible failure path returns a `NormError`
- Comments only where the WHY is non-obvious (spec cross-references, subtle invariants). Do not restate code

## Common Pitfalls

- Forgetting `--features cli` and wondering why CLI tests did not run
- Calling `.parse::<f64>()` on a numeric cell and losing precision — route numbers through the raw `serde_json::Number` representation enabled by `arbitrary_precision`
- Assuming section order in the output; `preserve_order` means the encoder must emit in declaration order, not alphabetical
- Treating an empty cell as `null` — it is absent key, which is different under `serde_json::Value::Object`
- Adding a MUST-reject rule without a fixture under `tests/fixtures/errors/` and a dedicated test in `tests/errors.rs`
- Extending `NormError` without updating `validate` aggregation tests when the new error can fire multiple times

## Commit and PR Guidelines

- Conventional-style prefixes are used in history (`feat:`, `fix:`, `docs:`, `test:`); follow the existing style
- One logical change per commit; keep fixture additions in the same commit as the parser or encoder change that motivates them
- Before opening a PR:
  - `cargo fmt --all`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features`
- Do not bump the crate version in a PR unless the PR is explicitly a release
- Do not edit `Cargo.lock` by hand

## Out of Scope

- Async APIs, streaming parse, memory-mapped I/O — the library is synchronous over `&str`
- File I/O in the library crate — stays in `src/bin/main.rs`
- Schema or type inference beyond what the spec defines
- Alternative serialisation targets (YAML, TOML, etc.)
