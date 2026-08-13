# GUIDE — Python under the Discipline, v0.1

**Status.** Beta; third T2 guide. Section structure is isomorphic to `GUIDE-RUST-v0.1.md` and `GUIDE-TYPESCRIPT-v0.1.md` — the guides are meant to be diffed across languages. Scope: only what the axioms require.

Framing note — the third point of the typology. Rust *enforces*; TypeScript *permits but compiles*; Python *trusts*: annotations are promises kept only by external tools, and at runtime everything is mutable, including an object's class. The discipline's instruments here are therefore the strictest in the set: an external checker as a hard gate, runtime parsing at every boundary, and the hardest bans on dynamism. Patterns dissolve via first-class functions and Protocols: Strategy = Protocol + injected callable; Visitor = union + `match` + `assert_never`; GoF Decorator = a wrapper object (and is *not* the same thing as a Python `@decorator` — the name collision deceives); Observer = an explicit event seam; Singleton = forbidden, and the module-level instance is precisely the Python form of the disease.

**Scope honesty.** This guide governs **cells** — long-lived system code. Exploratory notebooks, one-off scripts, and data-analysis scratch work are explicitly out of discipline scope; forcing them in would violate A2's economics. Framework surfaces (web routes, CLI entry points, fixtures) are boundary modules (§1) that *call* cells, not cells themselves.

---

## 0. Language baseline {#baseline}

- **Version floor:** Python 3.12 (PEP 695 native generics: `class Ok[T]`, `type Result[T, E] = …`); target the latest stable.
- **Type gate:** **pyright, `typeCheckingMode = "strict"`** — the single gating checker (two masters disagree; mypy MAY run advisory as a second evidence provider). Load-bearing settings: `reportUnnecessaryTypeIgnoreComment = "error"`, `enableTypeIgnoreComments = false` — together they ban bare `# type: ignore` and make every surviving `# pyright: ignore[ruleId]` **xfail-strict**: when the underlying error disappears, the suppression itself fails, forcing promotion (BROWNFIELD §4 at the type level).
- **Lint/format:** **ruff** (MIT; written in Rust — a pleasing T3 synergy) for both. Load-bearing rule families: `B` (bugbear), `UP`, `I`, `ASYNC`, `RUF` (incl. RUF100 unused-`noqa`), `PGH` (PGH003/PGH004 ban blanket `# type: ignore` / blanket `# noqa`) — every suppression carries its code or fails.
- **`Any` policy:** banned in cells (strict mode surfaces unknowns); `cast()` and coded ignores are legal only in **boundary modules** with a one-line justification. Prefer `object` + narrowing.
- **Workspace:** **uv** (MIT/Apache-2.0) — environments, `uv.lock` (reproducibility, A2), `[tool.uv.workspace]` as the pnpm/cargo-workspace analog. `pyproject.toml` is the single manifest.
- **Tests:** pytest with `xfail_strict = true` in `pyproject.toml` — the *origin* of the brownfield xfail-strict mechanism, applied globally: a bare lenient `xfail` is impossible; strict xfail markers are the in-source twin of `tests-baseline.json`, and `xtask test-gate` reconciles both. Property testing: **hypothesis** — **license note: MPL-2.0**, the Charter's case-by-case zone; verdict *acceptable*: test-only dependency, never linked into shipped artifacts. Flagged here so the exception is conscious, not accidental.
- **Boundary validation (parse, don't validate):** types are erased harder than in TS — there is not even a compile step. Every external input (network, disk, env, subprocess output) crosses through a typed parsing model at the boundary — **pydantic** (MIT) primary. Boundary models are the taggable schema units (the JTD parallel from vibevm): the model carries the spec edge; inferred types flow inward from it.

## 1. Cells {#cells}

A cell is a package (directory) behind one seam: `__init__.py` is the single public surface, with an honest, mandatory `__all__` exporting the seam implementation and nothing else.

- **Side-effect-free on import — the hardest rule in the hardest culture.** Module-level execution is idiomatic Python; in cells it is banned: top level admits imports, `def`/`class`, constants, `__all__`, `__specmap_scope__`, and `TYPE_CHECKING` blocks — no I/O, no global mutation, and **no registration-on-import** (the `@register`-at-import / autodiscovery pattern moves to the composition root). Enforced: T-syn top-level statement whitelist.
- **No sibling-cell imports** (R-002) — Python's circular-import hell makes the rule doubly valuable. Cross-seam *type* references via `if TYPE_CHECKING:` are fine and encouraged.
- **Platform capabilities are injected.** Cells never touch `os.environ`, filesystem I/O, HTTP clients directly — those are Protocol seams passed at construction. Time and randomness SHOULD also be injected (deterministic tests). Consequence: cell tests need no patching at all (§6).
- Values crossing seams SHOULD be `@dataclass(frozen=True, slots=True)` — immutable messages, cheap and explicit.
- **Promotion to a workspace member** when: heavy optional dependencies, independent publish boundary, or ~2 kLoC.

Cell manifest (decorator carrier, §5):

```python
@spec(implements="spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade", r=2)
@cell(seam="DepSolver", variant="sat", replaces="naive", flag="solver")
class SatDepSolver:
    def __init__(self, provider: DepProvider) -> None: ...
```

## 2. Seams {#seams}

- A seam is a `typing.Protocol` in core/seams — **Protocol over ABC**, deliberately: ABCs invite inheritance and template methods (hidden control flow, R-021); Protocol is pure shape — checkable structure with zero runtime coupling, at peace with duck-typing culture.
- **Composition over inheritance is a MUST at seams:** no behavior-bearing base classes, no behavior mixins in cells (MRO is action-at-a-distance).
- `@runtime_checkable` MAY be used for registry sanity asserts — with the documented caveat that it checks method *presence*, not signatures; pyright checks the signatures.
- Structural-typing caveat as in TS: accidental conformance is possible; a `ClassVar` brand MAY mark identity-critical seams; the registry remains the gatekeeper.

## 3. Registry and flags {#flags}

R-001 binding — flag at the seam, never in the veins:

```python
# src/registry.py — the only module reading selection flags and the only
# legal site of dynamic import (importlib with computed names).
def dep_solver(flags: Flags, provider: DepProvider) -> DepSolver:
    match flags.get("solver"):           # provenance: default | env | cli | lockfile
        case "sat":
            mod = importlib.import_module("cells.sat_dep_solver")   # lazy delivery
            return mod.SatDepSolver(provider)
        case _:
            return NaiveDepSolver(provider)                          # eager
```

- **Two tiers, never confused:** optional-dependency **extras** answer *"is the code in the environment"* — the cargo-feature analog; runtime flags answer *"is the cell selected"*. `try: import` guards live only in the registry, never in cells.
- Eager vs lazy cell loading (static import vs `importlib` in the registry) mirrors vibevm's delivery modes; lazy is a registry decision.
- **No DI frameworks, no module-level singleton wiring.** The `settings = Settings()`-at-import pattern is the Python singleton disease: config objects are constructed at the composition root and passed down. Explicit constructor injection + the registry `match` is the system's table of contents.

## 4. Errors as contract {#errors}

Python cannot type `raises` — the same A1 hole as TS, in a culture even more exception-centric. The split mirrors Rust's values-vs-panics:

- **Expected failures are values at seams.** Minimal core type, no framework dependency:

  ```python
  @dataclass(frozen=True, slots=True)
  class Ok[T]:  value: T
  @dataclass(frozen=True, slots=True)
  class Err[E]: error: E
  type Result[T, E] = Ok[T] | Err[E]
  ```

  Error objects carry `code` and the violated REQ URI (`spec: ClassVar[str]`); user-facing rendering appends the URI (PROP-014 §2.6).
- **EAFP stays legal *inside* a cell** — the official idiom is not repealed; the seam surface is where failure must be a value, because that is the only way the failure set becomes part of the checked interface.
- **`raise` is for invariant violations** (the panic analog); never raise bare `Exception`; chain with `raise … from e`.
- **Exhaustiveness:** closed sets are unions handled by `match` with `assert_never(x)` in the default arm — pyright turns Python's unchecked `match` into a compiler-verified one.
- **Async hygiene (MUST):** no un-awaited coroutines (pyright + ruff `ASYNC`); no fire-and-forget `create_task` — unreferenced tasks are garbage-collected mid-flight, Python's most treacherous async landmine; structured concurrency via `asyncio.TaskGroup` (+ `asyncio.timeout`) over bare `gather`/`create_task`.
- **Provenance note:** tracebacks resolve to real source lines natively — the release-map problem is easier than in JS; if shipping frozen/compiled distributions, retain line tables so the A1 chain survives.

## 5. specmark carrier {#specmark}

Decorators — and this is a deliberate asymmetry with the TS guide, where decorators were rejected: a Python decorator is a plain function with zero new semantics, costs one call at class creation, works on every `.py` (there is no untyped sibling language), and is parseable by T-syn while *also* attaching an introspectable `__specmark__` attribute — runtime tooling can read the edges, a capability Rust and TS carriers lack.

```
@spec(implements=<uri>, r=<N>)                  # one edge per decorator; stack them
@spec(deviates=<uri>, r=<N>, reason="…")        # reason mandatory
@verifies(<uri>, r=<N>)                          # on tests
__specmap_scope__ = ("<uri>", <N>)               # module-level inheritance
                                                 # (not __spec__ — importlib owns that name)
```

≤3 edges per item or split (same lint as the siblings).

## 6. Naming (R-020/R-021 bindings) {#naming}

- Canonical cell class name is computed: `{Variant}{Seam}` → `SatDepSolver`; hand-written names are linted against the manifest. Length free, ambiguity not.
- **Forbidden in cells regardless of elegance** — the Python theater list: metaclasses in domain code (infra crates only — the proc-macro parallel); `__getattr__`/`__getattribute__` dynamic dispatch; monkey-patching anything; **`unittest.mock.patch` / pytest `monkeypatch` in cell tests** — patching through the module graph is action-at-a-distance, and capability injection (§1) makes it unnecessary; import-time registration hooks (`__init_subclass__`, `__set_name__` registries) in domain code; `eval`/`exec`; `globals()`/`setattr`-loop object surgery; descriptors with side effects; star imports; mutable default arguments (ruff B006 — the classic).

## 7. Replacement protocol (R-040 binding) {#replacement}

A cell with `replaces=…` ships a differential oracle: hypothesis property tests asserting agreement with the old cell across the seam (documented-divergence list otherwise), `@verifies`-tagged. Snapshot artifacts (syrupy-class tools) follow the promotion protocol — CI never updates; local updates carry a debt/intent reference. With `xfail_strict = true` global, a healed known-failing test breaks the gate until promoted — the registry shrinks truthfully by construction.

## 8. Risk table (what conform must cover for Python) {#risks}

| Footgun | Rule | Tier |
|---|---|---|
| `Any` / `cast` / coded-ignore outside boundary modules | §0 | T-syn + pyright |
| bare `# type: ignore` / bare `# noqa` | §0 (PGH003/PGH004) | T-lex |
| un-awaited coroutine / unreferenced `create_task` | §4 | T-sem |
| import-time side effects or registration in cells | §1 | T-syn |
| cell importing a sibling cell | R-002 | T-syn |
| expected failure raised across a seam | §4 | T-sem |
| closed-union `match` without `assert_never` default | §4 | T-sem |
| `mock.patch` / `monkeypatch` in cell tests | §6 | T-syn |
| metaclass / `__getattr__` / subclass-hook registry in cells | §6 | T-syn |
| direct `os.environ` / file / network access in cells | §1 | T-syn |
| dynamic import outside the registry | §3 | T-syn |
| module-level singleton wiring (`X = X()` at import) | §3 | T-syn |
| mutable default argument | §6 (B006) | T-syn |
| flag read outside the registry | R-001 | T-syn |
| public export without own/inherited spec edge | PROP-014 §3.2-6 | T-syn + index |

## 9. Doc layer {#docs}

Google-style docstrings (ruff `D` family) on every tagged export: error codes and their REQ URIs, async semantics, edge cases, performance traps. The docstring is the human-facing detail layer — and, Python bonus, a runtime-introspectable one (`help()`); the spec stays thin; the ledger renders from both. Duplication between docstring and spec is a defect on the spec side.

---

**First carrier note.** The first Python cell under this guide is designated by the architecture itself — the conform engine's CPython `ast`/`symtable` sidecar (ENGINE §2): the process that asks Python about its own language. Self-hosting, third time.

*Any rule binding here without a corresponding conform check (or explicit `WISH` mark in the Charter rule record) by the first Python carrier milestone is removed rather than carried as aspiration.*
