# Tool Spec: `rust-ai-native-conform-frontend` — the Rust frontend for the language-neutral conform engine {#root}

<status stage="spec" state="done"/>

##status-line *Status: **SHIPPED with this package** — `crates/rust-ai-native-conform-frontend` (`id = "rust-syn"`) + `crates/rust-ai-native-conform` (binary **`rust-ai-native-conform`**), parsing `.rs` in-process with `syn`.* @impl/done

##RUST-IS-THE-PILOT-THE-OTHERS-ARE-PROJECTIONS *Rust is the pilot language: `go-ai-native-conform-frontend` and `typescript-ai-native-conform-frontend` are projections of the discipline this frontend proved first. This document is the last of the three surface specs to be written, and it is written to the form its own projections established.* @impl/done

##NO-SIDECAR-IS-THE-STRUCTURAL-DIFFERENCE *The one structural difference from both projections: Rust needs **no extractor sidecar**. Go spawns a stdlib-only `go-extract` and TypeScript a node process, each over an NDJSON bridge, because neither language parses itself from inside a Rust binary. Rust does — `syn` is a library — so the fact producer is a function call, not a process, and the bridge protocol, the content-addressed materialisation of the extractor, and the degraded-file note that protects against a crashing sidecar all have no counterpart here.* @impl/done

## 1. The division of labour with the native Rust tooling {#division}

##kind-line-division `req r1` @impl/done

##RUSTC-CARRIES-THE-TYPE-CORRECTNESS-HALF Rust's own toolchain carries the **type / correctness** half: `cargo build` (the compile gate), `cargo clippy` (the shipped lint census), `cargo fmt --check`, and the test suite as evidence providers. @spec/done

##THAT-HALF-IS-WELL-TYPED-AND-LOCALLY-SANE Those answer *"is this well-typed and locally sane?"* — the half the language does natively and well. @spec/done

##frontend-answers-the-structural-half-lead This frontend answers the **other** half — the *structural / architectural* rules no Rust tool expresses: @impl/done

- ##RULE-R-001-FLAG-SITES the cell-construction rule `R-001` (flag-sites): a cell's `<Type>::new(...)` constructor appears only in the selection registry module (`[rust] registry_file`), mounted only when both `registry_file` and `registry_gated_crate` are set; @impl/done
- ##RULE-R-002-CELL-ISOLATION `R-002` (cell isolation): a cell module imports seams and core only, never a sibling cell; @impl/done
- ##RULE-UNSAFE-GATE `unsafe-gate`: `unsafe` stays inside designated `[rust] audit_crates` or under a fn-grain `#[spec(deviates = …, reason = …)]` testimony (B-025: marked `DeviationAcknowledged`, visible in the IR, never failing the gate — not suppressed); @impl/done
- ##RULE-SEAM-HAS-DOCTEST `seam-has-doctest`: every public seam — a `pub` item at a gated crate's `src/lib.rs`, or a `pub trait` anywhere under `src/` — carries a compiled doctest; @impl/done
- ##RULE-PUB-DOCTEST `pub-doctest`: every public type seam (`struct` / `enum` / `trait` / `union`) in a `[rust] gated_pub_doctest` crate carries a compiled doctest or a `#[spec(documents)]` edge; @impl/done
- ##RULE-ERROR-ENUM-CITES-REQ `error-enum-cites-req`: a thiserror enum in a gated crate carries a `#[spec]` REQ edge — the attribute half of the Class-F seam-error contract; @impl/done
- ##RULE-ERROR-MESSAGE-CITES-REQ `error-message-cites-req`: a thiserror variant's `#[error("…")]` Display text carries a `spec://` REQ — the message half of the same contract; @impl/done
- ##RULE-CELL-HAS-ORACLE `cell-has-oracle`: every `#[cell]`-manifested type is referenced by an integration test in its crate (the differential / characterization oracle); @impl/done
- ##RULE-CELL-NAME-IS-COMPUTED `cell-name-is-computed`: a cell's type name follows the computed convention (Pascal(variant) + seam, B-038); @impl/done
- ##RULE-FILE-LENGTH `file-length`: a source file over the root `max_file_lines` budget (R3-003, position is a resource); @impl/done
- ##RULE-INVARIANT-COMMENT-POSITION `invariant-comment-position`: an invariant marker — a labeled, colon-bearing tag from the configured vocabulary, never a forceful word in prose — buried in the middle third of a long file, fed by the root `invariant_comment_markers` / `invariant_comment_min_file_lines` keys; @impl/done
- ##RULE-NO-UNWRAP-IN-DOMAIN `no-unwrap-in-domain`: `.unwrap()` / `.expect()` stays out of domain logic (test scope and fn-grain deviations honored); @impl/done
- ##RULE-AMBIENT-ENV `ambient-env`: `std::env::{var,var_os,set_var,remove_var}` reads stay at the composition root (`[rust] env_roots`) — an R-001 projection of the same flag-at-the-seam discipline; @impl/done
- ##RULE-DECLARED-TEST-MATRICES `declared-test-matrices`: a swept test matrix (a `2^n` bit-mask loop bound, or a ≥3-deep Cartesian nest of range `for` loops, in test context only) is generated, not declared (R-060); @impl/done

##RUST-RULES-DIFFER-FROM-THE-GO-PROJECTION-IN-SHAPE *Where this list differs from the Go projection (measured, not copied from the twin): Go folds its domain bans into one `RULE-BAN-CENSUS-AS-FACTS` umbrella, where Rust splits them into the individual rules `unsafe-gate`, `no-unwrap-in-domain`, and `ambient-env`; Go carries the seam-error contract as one rule (`go-seam-error-cites-req`, both halves), where Rust splits it into `error-enum-cites-req` (the attribute half) and `error-message-cites-req` (the message half); and the deviation escape hatch is not a standalone rule here — it is honored as `in_deviation` testimony stamped on the `UnsafeUse` / `UnwrapUse` / `EnvRead` facts (B-025). The `lint-suppression-needs-reason` rule is wired into the Rust gate too, but it is fed by SARIF-ingested `LintDiagnosis` facts (foreign linters such as clippy), not by this frontend's facts — so it belongs to the T-sem citation path (§5), not to this list.* @impl/done

##ONE-ENGINE-ONE-GRAMMAR-ONE-BASELINE Routing these through conform keeps **one rule engine, one finding grammar, one ratchet baseline** across all three languages, with the rules defined once over `conform_core::Fact` and fed by any frontend — a rule cannot drift between projections. @impl/done

## 2. What the frontend is {#extractor}

##kind-line-extractor `req r1` @impl/done

##RUST-FRONTEND-IS-A-FACT-PRODUCER A fact producer: parse a `.rs` file and emit the language-neutral fact stream the rules consume. @impl/done

##RUST-FRONTEND-IS-IN-PROCESS-SYN **In-process by construction** — `syn` parses the file inside the same binary, so there is no subprocess, no materialised extractor, no wire protocol and no toolchain requirement beyond the one already compiling the project. @impl/done

##UNPARSEABLE-INPUT-IS-TOLERATED An unparseable file yields **zero facts and no panic** — the same contract the sidecar frontends express as a `degraded` note, reached here by returning an empty vector. @impl/done

- ##FACT-KINDS **Fact kinds** (the rust-syn vocabulary): `Item` (a declared fn / struct / enum / trait with verbatim `spec(...)` / `cell(...)` / `verifies(...)` attribute text, the `pub` flag, and a doc-fence flag — the compiled-doctest candidate), `Import` (a `use` declaration: importing module → imported path), `Ctor` (a `<Type>::new(...)` construction site — the R-001 cell-construction signal), `UnsafeUse` (an `unsafe` block / `unsafe fn` / unsafe impl method, with `in_test` and fn-grain `in_deviation` scope), `ErrorVariant` (a `#[error("...")]` thiserror variant with its owning enum's attribute text — the Class-F signal), `FileMetrics` (whole-file physical line count, one per parsed file — the file-length signal), `UnwrapUse` (a `.unwrap()` / `.expect()` call site, with test and deviation scope), `EnvRead` (a `std::env::{var,var_os,set_var,remove_var}` access site — the ambient-env signal, with test and deviation scope), `InvariantComment` (a comment carrying an invariant marker, from a raw-text scan — `syn` drops plain `//` comments so the AST cannot supply them), and `TestSweep` (a swept-matrix signal in test context only — `bitmask` for a `2^n` loop bound, `nested-loops` for a ≥3-deep Cartesian nest of range `for` loops). The `TsUnsafe` / `TsEnvRead` / `TsSeamError` / `GoUnsafe` / `GoConformance` variants belong to the other frontends and are never emitted here; `LintDiagnosis` arrives via SARIF ingest, not this frontend. @impl/done
- ##RUST-PARSES-TWICE-AND-THE-DUPLICATION-IS-THE-COST **Rust parses twice, not once.** Go's twin claims "one extraction, two consumers" (the conform frontend eats `facts`, the specmap scanner eats `markers`, one parser). That does NOT hold for Rust: the specmap scanner (`core-ai-native-specmap::rscan::scan_source`) runs its OWN independent `syn::parse_file` pass over the same `.rs` text and emits `CodeItem` / `Edge` / `Warning` for the spec/code index, while this frontend's `syn::parse_file` pass emits `Fact`s for the rules. Two separate crates, two separate `syn` parses, two separate models — no shared parser, no shared vocabulary. The conformity Rust buys by parsing in-process everywhere does not extend to a single shared parse; the duplication is the recorded cost. @impl/done

## 3. The frontend crate {#frontend}

##kind-line-frontend `req r1` @impl/done

##FRONTEND-IMPLEMENTS-THE-ENGINE-TRAIT `rust-ai-native-conform-frontend` implements the engine's `Frontend` trait: an `id()` of `"rust-syn"`, a `version()` that bumps when the fact schema grows (retiring cache slots wholesale), and an `extract(file, package, module, text) -> Vec<Fact>` that parses in-process. @impl/done

##VERSION-IS-A-CACHE-KEY-NOT-A-RELEASE **The frontend version is a cache key, never a release number.** It is at **`"10"`** (frontend v10), and each bump retires every cached fact slot keyed by the old value. Each bump is a change to what the extraction EMITS — a new fact variant or field added (the history adds `ErrorVariant`, `FileMetrics`/`UnwrapUse`, `UnsafeUse`, `EnvRead`, `InvariantComment`, `TestSweep`, and the `is_pub`/`has_doctest` fields), or the scope of an existing fact narrowed (the marker vocabulary shrank to five labeled tags in v8; the nested-loop signal narrowed to range iterables in v10) — so the cached fact set from the old extraction no longer matches and must retire wholesale. @impl/done

##FACTS-ARE-CONTENT-ADDRESSED Facts are keyed `(file content-hash, frontend id+version)` in the engine's content-addressed store — a 1-file diff re-extracts 1 file. @impl/done

## 4. Topology: the `[rust]` policy section {#topology}

##kind-line-topology `req r1` @impl/done

##CONFORM-TOML-GAINS-A-RUST-SECTION `conform.toml` gains a `[rust]` section, the ten keys of the engine's `RustConfig`: `roots` (source roots to scan; a `<dir>/*` entry expands each subdirectory as one crate, any other entry is a literal crate dir; default `["crates/*"]`), `exclude_substrings` (a file whose repo-relative path contains any of these is skipped; default `["/generated/"]`), `gated` (the crates the Class-F/G gates apply to — the unit list, since Rust's gate unit is the crate; default `[]`), `[[rust.exempt]]` `{unit, reason}` (crates deliberately outside `gated`, each with a recorded reason; default `[]`), `gated_pub_doctest` (crates whose whole public type surface is gated for doctests; default `[]`), `audit_crates` (designated audit crates, exempt wholesale from the unsafe and ambient-env gates; default `[]`), `env_roots` (repo-relative files where reading the ambient environment is sanctioned; default `[]`), `registry_file` (the one legal cell-construction site, R-001; `None` disables R-001; default `None`), `registry_gated_crate` (the crate R-001 gates, meaningful only with `registry_file`; default `None`), and `[[rust.floor_disable]]` `{step, reason}` (floor steps this project disables, each with a recorded reason; default `[]`). @impl/done

##ROOT-KEYS-ARE-LANGUAGE-NEUTRAL-AND-LIVE-AT-THE-ROOT The **language-neutral** keys sit at the root of `conform.toml`, NOT under `[rust]`, because they model no language. The Rust rules read four: `max_file_lines` feeds `file-length`; `invariant_comment_markers` / `invariant_comment_min_file_lines` feed `invariant-comment-position`; and `sarif_reports` feeds the SARIF ingest whose `LintDiagnosis` facts the `lint-suppression-needs-reason` rule cites (the T-sem citation path, §5). What each root key means, and what it defaults to, is described once, in `ENGINE-CONFORM §6` (the policy file) — this surface names which ones the Rust rules read, not their values. The per-language tables `[rust]` / `[typescript]` / `[go]` are root keys too, each owning its own section; and nine retired flat root keys (`roots`, `exclude_substrings`, `gated_crates`, `gated_pub_doctest`, `audit_crates`, `env_roots`, `registry_file`, `registry_gated_crate`, `exempt`) are now loud tombstones — their presence parses but `Config::load` rejects each with a targeted move hint, never serde's generic unknown-field error. @impl/done

##EVERY-CRATE-GATED-OR-EXEMPT The every-unit-gated-or-exempt invariant is enforced by the engine on every check, exactly as for the sibling stacks — Rust's gate unit is the **crate** (Go's is the package, TypeScript's the cell — the B-029 ruling), so `[rust] gated` / `[[rust.exempt]]` is the expand-as-you-conform ratchet over crates. @impl/done

## 5. The honest note {#honesty}

##SYN-IS-A-PARSER-NOT-A-TYPE-CHECKER The structural gate is only as good as its facts, and `syn` is a PARSER, not a type checker: it sees syntax and attributes, not resolved types. @impl/done

##TYPE-DEPENDENT-RULES-ARE-OUT-OF-SCOPE Rules that would need type information (e.g. "this call's receiver is a seam type") are out of this tool's scope. @impl/done

##THE-DIVISION-IS-THE-THREE-TIER-SPLIT The division is deliberate — the same three-tier split (T-lex / T-syn / T-sem) `ENGINE-CONFORM` §1 defines; this frontend is the T-syn tier. @impl/done

##RUSTS-T-SEM-TIER-IS-CLIPPY-AND-THE-GAP-IS-A-TYPE-AWARE-LINT-LIBRARY **Rust's T-sem tier is not empty, and the gap is narrower than "no vehicle".** By the Go twin's own framing — "Go's T-sem tier is the toolchain itself" (`vet`, `staticcheck`, `exhaustive`) — the toolchain's lint engine IS the T-sem tier; Rust's toolchain lint engine is **clippy**, the direct parity analogue. And where the sibling stacks merely list their toolchain linters as evidence providers in prose, this engine reads clippy's verdicts back IN as `Fact::LintDiagnosis` via SARIF ingest (B-026: the root `sarif_reports` key, `sarif::load_reports`), so a Discipline rule may CITE a clippy diagnosis as the evidence for its own claim (`Fact::cites_lint`, the `lint-suppression-needs-reason` rule) — the deepest T-sem wiring of the three stacks, not an empty one. **The genuine, narrower gap** the draft was reaching for: there is no DISCIPLINE-AUTHORED type-aware lint library — no `dylint`-class vehicle carrying Discipline-specific type rules (none ships in this stack, which pins stable Rust 1.93 / edition 2024) — so the type-dependent rules the T-syn frontend cannot express (it sees syntax, not resolved types) stay out of the tool's scope. That gap is recorded rather than silent, and the grammar such a vehicle would speak (`violates REQ …; fix surface: …`) is already spoken by the conform rules themselves — and, through the citation path, by an ingested clippy diagnosis. @impl/done

## 6. What this document does NOT cover {#boundaries}

##THE-RULES-THEMSELVES-LIVE-IN-THE-ENGINE-SPEC The rules' semantics live in `ENGINE-CONFORM`, one definition for all three languages; this document covers only what the RUST side reads, emits and configures. @impl/done

##THE-GUIDE-IS-THE-AUDIENCE-DOCUMENT `GUIDE-AI-NATIVE-RUST.md` is the author-facing document — how to write code that passes; this one is the surface: what the tool reads and what it accepts. @impl/done
