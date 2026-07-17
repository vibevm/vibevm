specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#facts");

use serde::{Deserialize, Serialize};

/// One normalized fact (ENGINE-CONFORM §3). Variants carry exactly
/// what the checks consume; the schema grows with the rules (adding
/// a field or variant bumps the frontend version, which retires old
/// cache slots wholesale — facts never deserialize across schemas).
///
/// ```
/// use core_ai_native_conform::Fact;
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
    /// An `unsafe` block, `unsafe fn`, or unsafe impl method.
    /// `in_test` marks uses inside `#[cfg(test)]` modules or `#[test]`
    /// functions — carried as data, but unsafe-gate deliberately does
    /// NOT exempt them: unsoundness in tests is still unsoundness, and
    /// the audit crate serves tests too. `in_deviation` marks uses
    /// inside a fn carrying `#[spec(deviates = …, reason = …)]` — the
    /// recorded testimony the rule honors (ENGINE-CONFORM §4:
    /// a matching deviates record downgrades the finding). Fn-grain
    /// only, same as `UnwrapUse`.
    UnsafeUse {
        context: String,
        line: u32,
        in_test: bool,
        in_deviation: bool,
    },
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
    /// `in_deviation` marks sites inside a fn carrying
    /// `#[spec(deviates = …, reason = …)]` — recorded testimony the
    /// rule honors instead of flagging. Deliberately fn-grain: a
    /// deviates edge on a wider item (impl, struct, mod) records a
    /// different deviation, not unwrap amnesty for everything inside.
    UnwrapUse {
        method: String,
        line: u32,
        in_test: bool,
        in_deviation: bool,
    },
    /// A `std::env::{var,var_os,set_var,remove_var}` access site — the
    /// R-001 ambient-coupling signal (PROP-014's `ambient-env` rule).
    /// `in_test` marks sites inside `#[cfg(test)]` / `#[test]`; the rule
    /// scopes those out. `in_deviation` marks sites inside a fn carrying
    /// `#[spec(deviates = …, reason = …)]` — the recorded testimony the
    /// rule honors. Fn-grain, same as [`Fact::UnwrapUse`].
    EnvRead {
        method: String,
        line: u32,
        in_test: bool,
        in_deviation: bool,
    },
    /// A TypeScript `unsafe`-set occurrence
    /// (GUIDE-AI-NATIVE-TYPESCRIPT §8), produced by the `ts-tsc`
    /// frontend: `kind` is one of `any_type` / `as_cross` / `non_null`
    /// / `ts_ignore` / `ts_expect_error`. `reason` carries the
    /// `@ts-expect-error -- reason` text — the TS shape of
    /// `#[spec(deviates)]` testimony, honoured by `ts-unsafe-in-domain`
    /// the way the Rust rules honour a deviation record. `in_test`
    /// marks test files (`*.test.ts` / `*.spec.ts` / `__tests__/`),
    /// where the domain ban does not apply — file-grain, because TS
    /// test scoping is a file convention, not an attribute.
    TsUnsafe {
        kind: String,
        line: u32,
        in_test: bool,
        reason: Option<String>,
    },
    /// A Go ban-census occurrence (GUIDE-AI-NATIVE-GO §2, §5, §7),
    /// produced by the `go-extract` frontend: `kind` is one of
    /// `init_decl` / `blank_import` / `ambient_call` / `naked_go` /
    /// `error_string_match` / `t_skip` / `reasonless_suppression` /
    /// `seam_error_missing_req`. `reason` carries the text of a
    /// reasoned `//spec:deviates … reason="…"` covering the site (the
    /// Go shape of deviation testimony, honoured by
    /// `go-unsafe-in-domain` instead of flagged) or a suppression
    /// directive's own reason. `in_test` marks `_test.go` files —
    /// file-grain, because Go test scoping is a file convention.
    GoUnsafe {
        kind: String,
        line: u32,
        in_test: bool,
        reason: Option<String>,
    },
}

/// Facts of one source file, with its repo-relative path.
///
/// ```
/// use core_ai_native_conform::SourceFacts;
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
/// use core_ai_native_conform::{Fact, Frontend};
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
    /// Batch warm-up: the store calls this ONCE per run with every
    /// repo-relative file whose facts are not already cached, before
    /// any `extract` call. A frontend with per-invocation process
    /// overhead (`ts-tsc` spawns node) extracts the whole pending set
    /// here and serves `extract` from memory; in-process frontends
    /// (rust-syn) keep the no-op default.
    fn warm(&self, _pending_files: &[String]) {}
}
