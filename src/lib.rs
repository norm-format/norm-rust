//! NORM (Normalised Object Relational Model) codec.
//!
//! Public API:
//! - [`parse`] — NORM text → `serde_json::Value`, first error wins
//! - [`encode`] — `serde_json::Value` → NORM text, first error wins
//! - [`validate`] — NORM text → `Result<(), Vec<NormError>>`, collects all errors
//!
//! The library performs no file I/O. Callers are responsible for reading input
//! and writing output.

mod document;
mod encoder;
mod error;
mod lexer;
mod parser;

pub use encoder::encode;
pub use error::NormError;
pub use parser::{parse, validate};
