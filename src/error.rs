use thiserror::Error;

/// All errors produced by [`parse`](crate::parse), [`encode`](crate::encode),
/// and [`validate`](crate::validate).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NormError {
    /// UTF-8 BOM detected at the start of the input. NORM files must not contain a BOM.
    #[error("UTF-8 BOM detected; NORM files must not contain a BOM")]
    BomDetected,

    /// Null byte encountered anywhere in the input.
    #[error("null byte at line {line}")]
    NullByte { line: usize },

    /// The first non-comment line is not `:root` or `:root[]`.
    #[error("missing root declaration; first non-comment line must be :root or :root[]")]
    MissingRootDeclaration,

    /// A `:root` line is present but malformed (e.g. trailing garbage).
    #[error("invalid root declaration at line {line}")]
    InvalidRootDeclaration { line: usize },

    /// Section name does not match `[a-zA-Z_][a-zA-Z0-9_]*`.
    #[error("invalid section name {name:?} at line {line}")]
    InvalidSectionName { line: usize, name: String },

    /// Two sections share the same name.
    #[error("duplicate section name {name:?} at line {line}")]
    DuplicateSectionName { line: usize, name: String },

    /// Section was defined but never reached from the root during reference resolution.
    #[error("section {name:?} is unreachable from the root")]
    UnreachableSection { name: String },

    /// The same pk value appears in two rows across the document's table sections.
    #[error("duplicate pk {pk} at line {line}")]
    DuplicatePk { line: usize, pk: String },

    /// pk value is not a non-negative integer, or has a leading zero.
    #[error("invalid pk value {value:?} at line {line} (leading zeros not permitted)")]
    InvalidPk { line: usize, value: String },

    /// `@N` or `@name` reference does not match any row or section.
    #[error("unresolved reference {reference:?} at line {line}")]
    UnresolvedReference { line: usize, reference: String },

    /// Reference resolution revisits a row or section already in flight.
    #[error("circular reference detected involving {reference:?}")]
    CircularReference { reference: String },

    /// `:root` (object) declaration but the root section contains more than one data row.
    #[error(":root document has multiple rows in its root section (line {line})")]
    MultipleRootRows { line: usize },

    /// `:root` (object) declaration but the first section is declared as an array section.
    #[error(":root declaration but root section is an array section at line {line}")]
    ArraySectionAsRoot { line: usize },

    /// A blank row appears inside an array section, or an array row resolves to nothing.
    #[error("empty row in array section at line {line}")]
    EmptyArrayRow { line: usize },

    /// A data row appears outside any containing section.
    #[error("unexpected data row at line {line} (no containing section)")]
    UnexpectedDataRow { line: usize },

    /// The encoder received a scalar root; NORM roots must be an object or array.
    #[error("encoder: root JSON value must be an object or array, not a scalar")]
    ScalarRoot,
}
