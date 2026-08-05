# Command nodes in the map — B-019(б) {#root}

<status stage="spec" state="work" comment="boss design for BACKLOG B-019(б), captured 2026-08-06 on the M-B019B measurement; the owner's build ruling of 2026-08-01 covers parts а/б/в, and (а) is already built — this is (б) only, and it deliberately does not touch (в)"/>

##design-scope **Scope.** [`BACKLOG.md`](../../BACKLOG.md) `##B019-B` asks that a
command be an entity of the map rather than only a function, so that *«what
implements `vibe install`»* is answerable directly. The owner's ruling of
2026-08-01 is to build it and to build it **algorithmically, without an LLM**.
This design covers that part and nothing else: part (а), the code fingerprint,
is built and live on 916 of 932 items; part (в), the error-variant node, carries
an unresolved systems-boundary question the owner asked to be answered **before**
implementation, and it is not in here. @spec/done

##design-not-blocked **It does not ride the format change.**
[`map-format-change.md`](map-format-change.md) `##non-goal-command-nodes` already
ruled that (б) is a separate node type with its own extraction, not blocked by
that change and not carried by it. This document is the sibling that ruling
points at. @spec/done

## 1. What is measured today {#measured}

##m-eight-kinds **The committed map carries 932 code items in eight kinds** —
`mod` 415, `fn` 376, `enum` 62, `struct` 52, `schema-def` 9, `schema` 7, `impl` 7,
`trait` 4. No kind contains the substring `command`. Reproduce by tallying
`code_items[].item_kind` in `specmap.json`. @impl/done

##m-kind-is-an-open-string **`item_kind` is an open string, and this is the
load-bearing measurement.** `schemas/specmap.jtd.json:92-94` declares
`"item_kind": { "type": "string" }` with no `enum`, while its neighbours `verb`,
`spec_unit.kind`, `status` and `provenance` all carry one; the Rust model
(`generated/specmap/mod.rs:44-45`) types it `String`. So a new kind is **a new
value of an open field, not a schema bump** — the opposite of B-019(а), which
`map-format-change.md:72` correctly called a real bump because it added net-new
*fields*. @impl/done

##m-nothing-matches-on-kind **No production code matches or filters on
`item_kind`.** `explain.rs:158` prints it through, `explain.rs:243` and
`vibe-trace/src/fragment.rs:486` pass it through into JSON, and the only
equality tests are two in `jtd/tests.rs` that look for specific values rather
than assert a closed set. There is no kind→display-name table for code items.
A new value therefore breaks nothing downstream. @impl/done

##m-scanner-is-blind-to-derive **The Rust scanner cannot see `#[derive(...)]`
today.** `rscan.rs:89-134` `edges_from_attrs` is the only reader of attributes;
its `match` on the attribute path's last segment has exactly two non-wildcard
arms — `"spec"` (`:100`) and `"verifies"` (`:115`) — and `_ => {}` (`:130`)
swallows everything else. `grep -ni derive rscan.rs` returns nothing. @impl/done

##m-explain-target-is-open **`explain`'s target grammar is open, and this is the
second load-bearing measurement.** `explain.rs:199-204` branches on one string
prefix — `spec://` goes to `explain_unit`, everything else to `explain_symbol` —
and `explain_symbol` (`:123-150`) matches `codeItems[].symbol` exactly, then by
suffix, **without ever consulting `item_kind`**. There is no closed enum of
target kinds to extend. @impl/done

##m-all-three-stacks-are-clap **All three language stacks declare their commands
identically, because all three CLIs are Rust crates on clap** —
`rust-ai-native-cli/src/main.rs:27`, `typescript-ai-native-cli/src/main.rs:23`,
`go-ai-native-cli/src/main.rs:23`. What is per-language is the *code* extractor
each drives (`syn` in-process for Rust; the `ts-extract` and `go-extract`
sidecars for the other two) — not the command declaration. @impl/done

##m-twentyone-copies **The engine crate exists in 22 directories: one authored
and 21 copies** — the coexisting v0.7.0 slot plus 20 regenerated vendor and
`vibedeps/` snapshots. The authored one is
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/`.
Every engine edit is followed by `cargo xtask sync-engines` as its own step. @impl/done

## 2. How a command is recognised {#recognition}

##r-three-answers Three answers exist. **(A)** Recognise the framework: an enum
carrying a `Subcommand` derive is the command enum and each variant is a command.
**(B)** Make the author mark it — a new attribute or a `specmark` unit.
**(C)** Name the enum in `specmap.toml`. @spec/done

##r-a-wins **(A), and the reason is a law rather than a preference.** (B) and (C)
both fail `##WAL-C-A-NORM-WITHOUT-A-CHECKER-DRIFTS`: a subcommand added without
its marker, or without its config line, is simply absent from the map and nothing
says so — the shape this repository paid for on the licence norm, where one crate
of twenty fell out of a rule for months. (A) cannot drift, because the same
declaration that makes the command exist for the user is the one the scanner
reads. @spec/done

##r-not-a-framework-in-the-core **The objection that (A) puts a framework into a
language-neutral engine does not hold, because it puts it into
`rscan.rs` — the Rust-specific scanner, which already knows `syn`.** Knowing clap
is one more Rust-ecosystem fact in the layer where Rust-ecosystem facts belong;
the neutral core keeps knowing only that a code item has a kind. Per
`##m-all-three-stacks-are-clap` this single Rust reader already covers the host
and all three stack CLIs. A consumer project written in Go or TypeScript would
declare its commands in its own ecosystem's idiom, and its extractor belongs in
that language's existing sidecar — `##WAL-C-PARITY-IS-THE-INVARIANT-NOT-THE-CODE`:
parity is that a command is a node of the map, never that the code is the same. @spec/done

##r-both-spellings **Both derive spellings must be recognised.** This tree
carries `#[derive(Debug, Subcommand)]` (`crates/vibe-cli/src/cli.rs:94`) and
`#[derive(clap::Subcommand, Debug)]` (`:267`). A reader matching one form finds
part of the surface and reports a clean number — the failure mode
`##WAL-C-A-GREP-LIES-IN-BOTH-DIRECTIONS` names. The match is on the derive
path's **last segment**, exactly as `edges_from_attrs` already matches attribute
paths. @spec/done

##r-this-is-not-an-owner-fork **This is engineering judgement, not the owner's
court.** B-019's owner fork is (в)'s systems boundary and it is a different
question. @spec/done

## 3. What a command node carries {#node}

##n-no-new-fields A `code_item` with `item_kind = "command"` and the fields that
already exist. Nothing is added to the wire format. @spec/done

| field | value |
|---|---|
| ##n-symbol `symbol` @spec/done | the invocation path — `vibe install`, `vibe registry redirect` @spec/done |
| ##n-kind `item_kind` @spec/done | `"command"` — a new value of an open field (`##m-kind-is-an-open-string`) @spec/done |
| ##n-crate `crate_name` @spec/done | the crate declaring the enum @spec/done |
| ##n-span `file` / `line` / `end_line` @spec/done | the variant's span, same attribute-inclusive convention as every other item @spec/done |
| ##n-fingerprint `fingerprint` @spec/done | the variant's token stream, `tok1:<sha256>` — so «the command's declaration changed, re-check what it links to» works, which is (а)'s purpose applied to a new node @spec/done |

##n-symbol-is-what-a-human-types **The symbol is the invocation path and not the
Rust path, and that choice is what makes the row's question answerable.** With
`symbol = "vibe install"`, `vibe explain "vibe install"` resolves through the
existing `explain_symbol` path with **no change to `explain` at all**
(`##m-explain-target-is-open`). A Rust-path symbol would answer the same question
only after a translation step nobody asked for. @spec/done

##n-binary-name-is-declared The binary half of the path is read from the
`#[derive(Parser)]` root's `#[command(name = "…")]` — declared in this tree at
`crates/vibe-cli/src/cli.rs:47`. Where a root declares no name, clap's own
fallback applies and the extractor uses the same source clap does rather than
inventing one. @spec/work

##n-variant-name-rule The variant half is clap's own rename rule
(`Install` → `install`, `RedirectSync` → `redirect-sync`), and an explicit
`#[command(name = "…")]` **on a variant** wins over the derived form. The map's
string must be the string the user types, or the node answers a question nobody
asked. @spec/work

## 4. Where the extraction lives {#extraction}

##x-the-enum-is-an-ast-item **A command variant is reachable from the walk that
already runs.** `walk_items` visits `syn::Item::Enum(e)` at `rscan.rs:173`, and
`e.variants` hangs off that item; descending into variants is the same shape as
the descent into trait methods (`:182-188`) and impl methods (`:211-217`) that
the walker already performs. No second traversal of the tree is required.
*(The M-B019B measurement concluded that a command «cannot be fitted into
`rscan.rs`'s match» and needs a pass parallel to `jtd.rs`. The fact behind that
— a command is not a top-level `syn::Item` — is true; the conclusion does not
follow, because the enum that declares it is.)* @spec/done

##x-not-through-tag-item **It cannot ride `tag_item`, and that is the one real
obstacle.** `rscan.rs:144-147` returns early when an item carries no
`#[spec]`/`#[verifies]` edge, so today an item is recorded **only if it is
tagged**. A command exists whether or not anyone tagged it, so command nodes go
through `record_item` (`rscan.rs:46`), the unconditional recording path, which is
already there. @spec/done

##x-the-join-is-crate-wide **The binary name and the nesting are a crate-wide
join, and that is the design's only structural cost.** The `Parser` root lives in
one file (`cli.rs`), the group enums in others (`cli/registry.rs`,
`cli/progress.rs`, …), and `scan_source` (`rscan.rs:254`) is per-file. Three
relations must be collected during the walk and joined after it: *(i)* root
struct → binary name and the type of its `#[command(subcommand)]` field;
*(ii)* enum with a `Subcommand` derive → its variants and each variant's payload
type; *(iii)* args struct → the type of its own `#[command(subcommand)]` field,
where it has one. `scan_workspace` (`:313`) already accumulates across files, so
the join is a post-pass over state it already holds. @spec/work

##x-budget-does-not-choose-the-shape **`rscan.rs` is 511 lines against the
600-line budget, so this lands as a submodule the scanner calls — and the budget
is the reason for the file, never for the design.**
`##WAL-C-FILE-BUDGET-DOES-NOT-CHOOSE-A-TYPE`: a length budget may decide where
code sits and may not decide what it is. Any new file carries
`specmark::scope!(…)` in its crate's own form, or the panel's self-trace reports
its helpers as orphans. @spec/done

## 5. The landing cut {#cut}

##cut-1 **Slice 1 — top-level commands of one binary.** The derive reader, the
`record_item` path, the root-to-enum join, `symbol` as `<binary> <command>`.
Acceptance is a number: the host's map gains **29** command nodes — the count the
surfaces census
([`g6-b047-surfaces-census.md`](../../campaigns/packages-2026-09/harvest/g6-b047-surfaces-census.md))
established as the top-level surface, one per variant of `pub enum Command`. @spec/plan

##cut-2 **Slice 2 — nesting.** A variant whose payload type carries its own
`#[command(subcommand)]` yields commands one level deeper, with the parent's path
as their prefix. Acceptance is the census's other number: **68** subcommands
across the ten group variants. If the crate-wide join proves to cost more than
this slice can carry, slice 1 stands alone and says what it covers — a partial
landing that names its perimeter beats a whole one that waits. @spec/plan

##cut-3 **Slice 3 — the acceptance, which is expected to need no code.**
`vibe explain "vibe install"` must answer, and per `##m-explain-target-is-open`
the path is already open. The slice is a test that would have failed before
slice 1, plus the map regeneration. If it turns out that `explain` does need a
change, that is a measurement contradicting `##m-explain-target-is-open` and it
is reported as such rather than absorbed. @spec/plan

##cut-vendor **Every slice that edits the engine ends with
`cargo xtask sync-engines` as its own step** (`##m-twentyone-copies`), and a slice
that adds a `.rs` file to an engine crate ends with `cargo xtask specmap` in the
same landing — a new `scope!` unit moves the committed map. @spec/done

## 6. Non-goals {#non-goals}

##ng-not-v **Not part (в).** The error-variant node's systems boundary — whether
`specmap` extracts the data itself, reads `conform`'s, or the two are joined only
at query time — is the owner's to answer before implementation, by his own
requirement recorded in `##B019-V`. Nothing here presumes an answer. @spec/done

##ng-not-a-schema-bump **Not a schema bump.** If a build measures that
`item_kind` is closed somewhere this design did not find, that is a refusal to be
reported with the line, not a redesign to be improvised. @spec/done

##ng-not-the-spec-side-revisions **Not the spec-side half of (а).** Revision
marks on the ~80 spec sections that error messages cite are a separate lifetime
and are untouched here. @spec/done

##ng-not-other-languages **Not command extraction for consumer projects in Go or
TypeScript.** Their CLIs would declare commands in their own idiom and their
extractors belong in the sidecars those languages already ship. This design
covers the Rust+clap surface, which per `##m-all-three-stacks-are-clap` is the
host and all three stack CLIs. @spec/done
