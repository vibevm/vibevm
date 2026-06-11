use serde::{Deserialize, Serialize};

specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#facts");

/// One normalized fact (ENGINE-CONFORM §3). Variants carry exactly
/// what the checks consume; the schema grows with the rules (adding
/// a field or variant bumps the frontend version, which retires old
/// cache slots wholesale — facts never deserialize across schemas).
///
/// ```
/// use conform_core::Fact;
///
/// let fact = Fact::Item {
///     kind: "fn".into(),
///     symbol: "x::solve".into(),
///     line: 4,
///     attrs: vec![],
///     is_pub: true,
///     has_doctest: false,
/// };
/// assert!(matches!(fact, Fact::Item { is_pub: true, .. }));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "fact", rename_all = "snake_case")]
pub enum Fact {
    /// A declared item with its attributes (verbatim attribute text).
    Item {
        kind: String,
        symbol: String,
        line: u32,
        attrs: Vec<String>,
        /// `pub` at the item itself (visibility of the declaration,
        /// not reachability) — the seam signal for Class-G rules.
        #[serde(default)]
        is_pub: bool,
        /// The item's doc comment carries at least one fenced code
        /// block — a compiled doctest candidate (Class G).
        #[serde(default)]
        has_doctest: bool,
    },
    /// A `use` declaration: importing module → imported path.
    Import {
        from_module: String,
        to_path: String,
        line: u32,
    },
    /// A `<Type>::new(...)` construction site — the R-001 signal.
    Ctor { type_name: String, line: u32 },
    /// An `unsafe` block or `unsafe fn` body.
    UnsafeUse { context: String, line: u32 },
    /// A `#[error("...")]`-carrying enum variant (thiserror) with the
    /// enum's own attribute text — the Class-F diagnostics signal.
    ErrorVariant {
        enum_symbol: String,
        variant: String,
        message: String,
        line: u32,
        /// Attributes of the OWNING enum (where the REQ edge lives).
        enum_attrs: Vec<String>,
    },
    /// Whole-file metrics, one per parsed file — the guide §2
    /// "position is a resource" signal (file-length budget).
    FileMetrics { lines: u32 },
    /// A `.unwrap()` / `.expect(...)` call site. `in_test` marks call
    /// sites inside `#[cfg(test)]` modules or `#[test]` functions,
    /// where the ban does not apply (GUIDE-AI-NATIVE-RUST §6).
    UnwrapUse {
        method: String,
        line: u32,
        in_test: bool,
    },
}

/// Facts of one source file, with its repo-relative path.
///
/// ```
/// use conform_core::SourceFacts;
///
/// let sf = SourceFacts {
///     file: "crates/x/src/lib.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![],
/// };
/// assert_eq!(sf.crate_name, "x");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFacts {
    pub file: String,
    /// The crate directory name the file belongs to.
    pub crate_name: String,
    pub facts: Vec<Fact>,
}

/// A fact producer for one language (ENGINE-CONFORM §2). T-syn for
/// Phase 4; the trait carries id+version so the store key changes when
/// the frontend does.
///
/// The canonical implementation shape:
///
/// ```
/// use conform_core::{Fact, Frontend};
///
/// struct NullFrontend;
/// impl Frontend for NullFrontend {
///     fn id(&self) -> &'static str { "null" }
///     fn version(&self) -> &'static str { "1" }
///     fn extract(&self, _f: &str, _c: &str, _m: &str, _t: &str) -> Vec<Fact> {
///         Vec::new()
///     }
/// }
/// assert!(NullFrontend.extract("f.rs", "x", "x", "fn a() {}").is_empty());
/// ```
pub trait Frontend {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    /// Extract facts from one file. `module` is the module path the
    /// engine computed for it.
    fn extract(&self, file: &str, crate_name: &str, module: &str, text: &str) -> Vec<Fact>;
}
