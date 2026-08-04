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
    /// `seam_error_missing_req` / `seam_error_message_no_req`. `reason`
    /// carries the text of a reasoned `//spec:deviates … reason="…"`
    /// covering the site (the Go shape of deviation testimony, honoured
    /// by `go-unsafe-in-domain` instead of flagged) or a suppression
    /// directive's own reason. `in_test` marks `_test.go` files —
    /// file-grain, because Go test scoping is a file convention.
    ///
    /// The two `seam_error_*` kinds are the structure half
    /// (`seam_error_missing_req` — the error type carries no `Spec`
    /// field) and the message half (`seam_error_message_no_req` — its
    /// `Error()` renders no `spec://`); both are consumed by the
    /// dedicated `go-seam-error-cites-req` rule, not the umbrella.
    GoUnsafe {
        kind: String,
        line: u32,
        in_test: bool,
        reason: Option<String>,
    },
    /// A Go compile-time conformance assertion
    /// `var _ <seam> = (*<Impl>)(nil)` (GUIDE-AI-NATIVE-GO §2, the
    /// «conformance is made loud» idiom), produced by the `go-extract`
    /// frontend. `seam` is the interface the type satisfies, `impl_type`
    /// the implementing type, `line` the assertion line. `in_test` marks
    /// `_test.go` files. Consumed by `go-conformance-assertion`, the
    /// absence-check that fires when a cell declares no such assertion.
    GoConformance {
        seam: String,
        impl_type: String,
        line: u32,
        in_test: bool,
    },
    /// A TypeScript environment/config read site — `process.env.X`,
    /// `process.env["X"]`, or `import.meta.env.X` — produced by the
    /// `ts-tsc` frontend. This is the TS shape of the "flags/config are
    /// read once at the composition root" signal
    /// (GUIDE-AI-NATIVE-TYPESCRIPT §7, B-039). `source` names the read
    /// base (`"process.env"` / `"import.meta.env"`) so the finding points
    /// at the actual exterior handle; `in_test` marks test files, where
    /// the domain rule does not apply — file-grain, same as
    /// [`Fact::TsUnsafe`]. Consumed by `ts-flag-sites`, the TS-native
    /// twin of Rust's R-001: an env read is legal only in the single
    /// file named by `[typescript] composition_root`.
    TsEnvRead {
        source: String,
        line: u32,
        in_test: bool,
    },
    /// A TypeScript discriminated-union error type alias `E`
    /// (GUIDE-AI-NATIVE-TYPESCRIPT §6, the «failure on a seam is a typed
    /// value» contract), produced by the `ts-tsc` frontend. `symbol` is
    /// the alias name; `cites_req` is the extractor's computed flag —
    /// whether the union cites a `spec://` REQ (a JSDoc
    /// `@implements`/`@documents` marker on the alias OR a `spec://`
    /// substring in a variant member); `line` is the alias line.
    /// `in_test` marks test files (`*.test.ts` / `*.spec.ts` /
    /// `__tests__/`). Consumed by `ts-seam-error-cites-req`, the TS
    /// twin of the Rust/Go seam-error rules.
    TsSeamError {
        symbol: String,
        cites_req: bool,
        line: u32,
        in_test: bool,
    },
    /// A comment carrying an invariant marker
    /// (`INVARIANT:` / `WARNING:` / `PANICS:` / `MUST:` / `NEVER:`, …),
    /// produced by a comment-walking frontend. `marker` is the normalized
    /// marker exactly as recorded in the config vocabulary; `line` is the
    /// comment's line; `in_test` marks a comment in test context, where
    /// the position rule does not apply. Consumed by
    /// `invariant-comment-position` (R3-003 "position is a resource"): an
    /// invariant marker that lands in the middle third of a long file is
    /// buried where a reader pages past it.
    InvariantComment {
        marker: String,
        line: u32,
        in_test: bool,
    },
    /// A swept test matrix — a test whose cases are GENERATED by a full
    /// enumeration rather than DECLARED as data (R-060). Produced by every
    /// frontend ONLY in test context (a `#[cfg(test)]`/`#[test]` site for
    /// rust-syn, a `_test.go` file for go-extract, a `*.test.ts`/
    /// `*.spec.ts` file for ts-tsc): outside tests the rule is not about
    /// this. `kind` is the syntactic signal that fired — `"bitmask"` (a
    /// loop bound sweeps `2^n`: `1 << n` / `2 ** n` / `math.Pow(2, n)`) or
    /// `"nested-loops"` (a Cartesian product of ≥ 3 nested GENERATED-axis
    /// loops — a Rust range `for i in 0..n`, a Go/TS C-style `for`; a loop
    /// over a DECLARED collection/array/constant does NOT count, so a nest
    /// of those is compliant). `line` is the loop's line; `detail` is the
    /// short machine detail — the bound text for a bitmask (`"1 << n"`) or
    /// the nesting depth for nested loops (`"3"`). Consumed by
    /// `declared-test-matrices`, the rule that a table of cases iterated
    /// once (or a closed set exhausted by nesting collection loops) is
    /// compliant and a generated exponent is not.
    TestSweep {
        kind: String,
        line: u32,
        detail: String,
    },
    /// A diagnosis from a FOREIGN linter — another tool's verdict on the
    /// codebase, read back IN as a fact (B-026 SARIF ingest). The Discipline
    /// quotes a foreign linter rather than reinventing it: clippy, eslint,
    /// golangci-lint already find what they find, and a Discipline rule may
    /// CITE one of their diagnoses as the evidence for its own claim
    /// (`check: { tool, rule_id, status }` — see [`Fact::cites_lint`]).
    ///
    /// Produced NOT by a per-language frontend but by [`sarif::ingest`]
    /// (`crate::sarif`) reading a SARIF 2.1.0 report a flora step deposited
    /// — so this is the one fact whose origin is a file a tool wrote, not
    /// source the engine parsed. `tool` is the SARIF
    /// `runs[].tool.driver.name`; `rule_id` is the result's `ruleId`;
    /// `file`/`line` come from the first result location's
    /// `physicalLocation` (`artifactLocation.uri` / `region.startLine`);
    /// `message` is the result's `message.text`.
    ///
    /// `suppressed` carries the result's `suppressions`: a suppressed
    /// diagnosis is the foreign-linter shape of "known and accepted in
    /// source" — exactly what [`FindingStatus::DeviationAcknowledged`]
    /// (B-025) was built for. A rule that surfaces a suppressed diagnosis
    /// stamps the finding `DeviationAcknowledged`, so it stays visible in
    /// the IR/SARIF but never fails the gate (the existing gate-inert
    /// path in `baseline::diff` / `baseline::freezeable`, reused for
    /// free — no new status). `reason` is the suppression's
    /// `justification` text when the report carried one.
    LintDiagnosis {
        tool: String,
        rule_id: String,
        file: String,
        line: u32,
        message: String,
        suppressed: bool,
        reason: Option<String>,
    },
}

impl Fact {
    /// A compact, visualizer-facing rendering of this fact — the value
    /// [`Finding::evidence`](crate::Finding::evidence) carries. Names the
    /// variant and its salient discriminator(s) (the kind/symbol/carrier
    /// a reader needs, plus the `in_test`/`in_deviation`/`reason` flags a
    /// deviation view needs), WITHOUT the prose that would bloat the
    /// finding record. One line, no delimiters a tool must parse: it is a
    /// label for a human/dashboard, not a structured channel (B-025 —
    /// every signal stays visible in the IR).
    pub fn summary(&self) -> String {
        match self {
            Fact::Item { kind, symbol, .. } => format!("Item({kind}:{symbol})"),
            Fact::Import { to_path, .. } => format!("Import({to_path})"),
            Fact::Ctor { type_name, .. } => format!("Ctor({type_name})"),
            Fact::UnsafeUse {
                context,
                in_test,
                in_deviation,
                ..
            } => {
                format!("UnsafeUse({context},test={in_test},dev={in_deviation})")
            }
            Fact::ErrorVariant {
                enum_symbol,
                variant,
                ..
            } => {
                format!("ErrorVariant({enum_symbol}::{variant})")
            }
            Fact::FileMetrics { lines } => format!("FileMetrics({lines})"),
            Fact::UnwrapUse {
                method,
                in_test,
                in_deviation,
                ..
            } => {
                format!("UnwrapUse({method},test={in_test},dev={in_deviation})")
            }
            Fact::EnvRead {
                method,
                in_test,
                in_deviation,
                ..
            } => {
                format!("EnvRead({method},test={in_test},dev={in_deviation})")
            }
            // TS/Go facts carry `reason` on the variant, but the reason
            // text is NOT echoed here — it rides on the finding's status
            // for acknowledged deviations (where it belongs) and would
            // only bloat the evidence label for a live finding.
            Fact::TsUnsafe { kind, in_test, .. } => format!("TsUnsafe({kind},test={in_test})"),
            Fact::GoUnsafe { kind, in_test, .. } => format!("GoUnsafe({kind},test={in_test})"),
            Fact::GoConformance {
                impl_type, seam, ..
            } => {
                format!("GoConformance({impl_type}:{seam})")
            }
            Fact::TsEnvRead {
                source, in_test, ..
            } => format!("TsEnvRead({source},test={in_test})"),
            Fact::TsSeamError {
                symbol, cites_req, ..
            } => {
                format!("TsSeamError({symbol},cites_req={cites_req})")
            }
            Fact::InvariantComment { marker, .. } => format!("InvariantComment({marker})"),
            Fact::TestSweep { kind, detail, .. } => format!("TestSweep({kind}:{detail})"),
            // A foreign diagnosis: name its tool + rule + suppression
            // status (the reason text rides on the finding's status for
            // acknowledged ones, same posture as TsUnsafe/GoUnsafe).
            Fact::LintDiagnosis {
                tool,
                rule_id,
                suppressed,
                ..
            } => format!("LintDiagnosis({tool}:{rule_id},suppressed={suppressed})"),
        }
    }

    /// B-026 — the citation dictionary: is this fact a diagnosis a
    /// FOREIGN linter (`tool`) produced under rule `rule_id`? A Discipline
    /// rule calls this to say «this diagnosis confirms my claim» — quoting
    /// the foreign linter instead of reinventing its check. The form is
    /// `check: { tool, id, status }`: `tool` + `rule_id` name the foreign
    /// verdict, and `suppressed` filters by the diagnosis's status —
    /// `None` accepts either, `Some(true)` only an acknowledged
    /// (suppressed) one, `Some(false)` only a live one.
    ///
    /// ```
    /// use core_ai_native_conform::Fact;
    ///
    /// let live = Fact::LintDiagnosis {
    ///     tool: "clippy".into(), rule_id: "clippy::unwrap_used".into(),
    ///     file: "src/a.rs".into(), line: 4, message: "used unwrap".into(),
    ///     suppressed: false, reason: None,
    /// };
    /// let ack = Fact::LintDiagnosis {
    ///     tool: "clippy".into(), rule_id: "clippy::unwrap_used".into(),
    ///     file: "src/a.rs".into(), line: 9, message: "used unwrap".into(),
    ///     suppressed: true, reason: Some("FFI boundary".into()),
    /// };
    /// // {tool, id} match; status narrows it.
    /// assert!(live.cites_lint("clippy", "clippy::unwrap_used", None));
    /// assert!(live.cites_lint("clippy", "clippy::unwrap_used", Some(false)));
    /// assert!(!live.cites_lint("clippy", "clippy::unwrap_used", Some(true)));
    /// assert!(ack.cites_lint("clippy", "clippy::unwrap_used", Some(true)));
    /// // Wrong tool or id: no match, regardless of status.
    /// assert!(!live.cites_lint("eslint", "clippy::unwrap_used", None));
    /// assert!(!live.cites_lint("clippy", "clippy::something_else", None));
    /// // A source fact is never a foreign diagnosis.
    /// assert!(!Fact::Ctor { type_name: "X".into(), line: 1 }
    ///     .cites_lint("clippy", "clippy::unwrap_used", None));
    /// ```
    pub fn cites_lint(&self, tool: &str, rule_id: &str, suppressed: Option<bool>) -> bool {
        match self {
            Fact::LintDiagnosis {
                tool: t,
                rule_id: r,
                suppressed: s,
                ..
            } => t == tool && r == rule_id && suppressed.is_none_or(|want| want == *s),
            _ => false,
        }
    }
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
