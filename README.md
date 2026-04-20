# norm-codec

A Rust parser and encoder for the [NORM](https://github.com/norm-format) (Normalised Object Relational Model) data format. Provides a library crate for embedding NORM in your applications and an optional `norm` CLI for converting between NORM and JSON.

[![Crate](https://img.shields.io/crates/v/norm-codec.svg)](https://crates.io/crates/norm-codec)
[![License: MPL-2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](LICENSE)

## Features

- Parse NORM text into `serde_json::Value`
- Encode `serde_json::Value` into NORM text
- Validate NORM documents and collect every error, not just the first
- Preserves numeric precision and key order via `serde_json` features
- Pure Rust, no `unsafe`, no file I/O in the library
- Optional feature-gated `norm` CLI built on `clap`
- Tracks NORM spec v0.1

## Installation

### Library

Add `norm-codec` to your `Cargo.toml`:

```toml
[dependencies]
norm-codec = "0.1"
```

### CLI

Install the `norm` binary from source:

```sh
cargo install norm-codec --features cli
```

Or build it from a local checkout:

```sh
cargo build --release --features cli
./target/release/norm --help
```

## Library Usage

```rust
use norm_codec::{encode, parse, validate, NormError};

fn main() -> Result<(), NormError> {
    let input = "\
:root
:user
pk,name,email
1,Ada,ada@example.com
";

    // NORM → JSON
    let value = parse(input)?;
    println!("{}", serde_json::to_string_pretty(&value).unwrap());

    // JSON → NORM
    let norm = encode(&value)?;
    println!("{norm}");

    // Aggregate validation
    if let Err(errors) = validate(input) {
        for err in errors {
            eprintln!("{err}");
        }
    }

    Ok(())
}
```

### API

| Function | Signature | Behaviour |
|----------|-----------|-----------|
| `parse` | `fn parse(input: &str) -> Result<Value, NormError>` | Returns the first error encountered. |
| `encode` | `fn encode(value: &Value) -> Result<String, NormError>` | Returns the first error encountered. |
| `validate` | `fn validate(input: &str) -> Result<(), Vec<NormError>>` | Collects every error for reporting. |

The library performs no file I/O. Callers own reading input and writing output.

## CLI Usage

The `norm` binary reads from a file argument or stdin, and writes to stdout. Errors are written to stderr. Exit code is `0` on success and `1` on failure.

```sh
# NORM → JSON (pretty-printed by default)
norm parse input.norm

# NORM → JSON (compact)
norm parse --compact input.norm

# JSON → NORM
norm encode input.json

# Validate, print every error to stderr
norm validate input.norm

# Pipe through stdin
cat input.norm | norm parse
```

## Development

```sh
cargo build                    # library only
cargo build --features cli     # library + binary
cargo test                     # run all tests
cargo test --test parse        # one integration test file
cargo clippy                   # lint
cargo fmt                      # format
```

Integration tests live in `tests/` and share fixtures under `tests/fixtures/`. Error-rejection rules are covered one-test-per-rule in `tests/errors.rs`.

## Project Layout

```
src/
  lib.rs       public API re-exports
  error.rs     NormError enum
  lexer.rs     line-level tokeniser
  document.rs  internal document model
  parser.rs    NORM → serde_json::Value
  encoder.rs   serde_json::Value → NORM
  bin/main.rs  CLI (feature = "cli")
tests/
  parse.rs, encode.rs, roundtrip.rs, errors.rs, validate.rs, cli.rs
  fixtures/    .norm and .json pairs
```

See [`AGENTS.md`](AGENTS.md) for an architectural overview.

## Contributing

Issues and pull requests are welcome. Before opening a PR:

1. Run `cargo fmt` and `cargo clippy`
2. Ensure `cargo test` passes
3. For any new MUST-reject rule, add a fixture under `tests/fixtures/errors/` and a dedicated test case in `tests/errors.rs`

## License

Licensed under the Mozilla Public License 2.0. See [LICENSE](LICENSE) for the full text.
