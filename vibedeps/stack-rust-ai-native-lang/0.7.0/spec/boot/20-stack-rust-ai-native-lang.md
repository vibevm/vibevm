# AI-Native Rust (Discipline v0.2) — boot snippet {#root}

<status stage="impl" state="done"/>

##RUST-CODE-FOLLOWS-THE-RUST-GUIDE Rust code in this project follows the AI-Native Rust guide
(`spec/rust/GUIDE-AI-NATIVE-RUST.md` in this package). @impl/done

##READ-THE-GUIDE-FOR-STRUCTURE Read the guide when
authoring or reviewing structure; per-edit work needs only the card
whose trigger fires. @impl/done

##CARD-REGISTRY-FOR-RUST Card registry for Rust: `spec/cards/INDEX.md` in this package (trigger → card;
the nine executable scaffolds A–I in their Rust shape). @impl/done

##STACK-SHIPS-ITS-OWN-CARDS-PROJECTION This stack ships
its own `spec/cards/` projection — the weak-reader runtime surface for a `.rs`
edit is a Rust card's Band-3 ops block, never another language's. @impl/done

##standing-rules-lead Standing rules at the surface level: @impl/done

- ##RULE-ORDINARY-IDIOMATIC-RUST-SURFACE Ordinary idiomatic Rust at the token level — no invented dialect.
  Strictness lives in the envelope: newtypes/typestate at seams,
  runnable contracts at use sites, `#[spec]` metadata, per-cell
  fast verification (`cargo test -p <cell>`, < ~60s). @impl/done
- ##RULE-CELLS Cells: one cell = one file-set, single registration point; cells
  import seams + core only, never sibling cells. Ambient coupling is
  forbidden. @impl/done
- ##RULE-ERROR-ENUM-PER-LAYER One `thiserror` enum per layer; error messages cite the violated
  `spec://` REQ and a fix surface. `unwrap`/`expect` in domain logic
  is forbidden by default — escape hatch is
  `#[spec(deviates, reason)]` with machinery. @impl/done
- ##RULE-DOCTEST-PER-SEAM Every public seam carries one compiled doctest of canonical use.
  Replacing a non-trivial cell requires a differential oracle. @impl/done
- ##RULE-UNIFORMITY-IS-LOAD-BEARING Uniformity is load-bearing: one idiom per operation; exceptions
  are marked, or they propagate as false training signal. @impl/done

##shipped-toolchain-lead The shipped toolchain (this stack materialises it; no dev tree needed): @impl/done

- ##TOOLCHAIN-RUST-AI-NATIVE-UMBRELLA `rust-ai-native` — `init` (bootstrap policies + registries), `floor`
  (fmt→test→clippy→conform→specmap→test-gate, one exit code), `health`
  (the sweep's fact collector), `test-gate` / `tripwire` / `trace` /
  `fast-loop` / `codemod` / `ledger`; @impl/done
- ##TOOLCHAIN-NARROW-ENGINES plus the narrow `rust-ai-native-conform` and
  `rust-ai-native-specmap` engines, @impl/done
- ##TOOLCHAIN-AGENTIC-TYPE-ORACLE and the agentic type oracle `rust-ai-native-tcg`
  (also served over MCP by `mcp:org.vibevm.ai-native/rust-ai-native-mcp` — PROP-027)
  (persistent enriching `serve` relay + one-shot
  `validate`/`scope`/`complete`/`type`/`bench`: check an edit against
  in-memory overlays BEFORE writing it, answered by the CONSUMER's own
  rust-analyzer with the SAME conform rules as the gate — GUIDE §12, §13
  move 8; prerequisite `rustup component add rust-analyzer`; honesty:
  the oracle approximates, the floor stays the truth). @impl/done

##RUN-VIBE-NATIVELY-FROM-PATH-OR-IN-PLACE Run vibe-natively
(`vibe bin exec rust-ai-native -- <args>` — PROP-025 lockfile
dispatch; `vibe bin build` pre-builds), from PATH (`cargo install
--path vibedeps/<stack-slot>/crates/rust-ai-native-cli`), or in place
via `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
rust-ai-native-cli --bin rust-ai-native -- <args>`. @impl/done

##wiring-and-sweep-pointers Wiring recipe:
GUIDE §13; sweep idioms: GUIDE §14. @impl/done

##PROCEDURES-AS-AGENT-SKILLS Procedures as agent skills:
`/rust-ai-native-sweep` (recurring), `/rust-ai-native-terraform` (brownfield
adoption) — `vibe skill install` projects them. @impl/done
