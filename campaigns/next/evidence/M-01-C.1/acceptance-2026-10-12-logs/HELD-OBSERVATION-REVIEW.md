# M-01-C.1 held observation integration review

2026-10-12, initial source observations at 04:40:42 UTC. Read-only; no tests/builds or source effects. B.2 acceptance is retained as recorded in its receipt. Active C.1 code may change after these reads; this is integration-seam evidence, not acceptance.

**SafeFS already provides the core single-observation buffered primitive. The smallest actual consumer integration is to replace the resolver's preflight-plus-two-ambient-reads with a bounded SafeFS snapshot, and derive raw digest and parsing/projection input from that same returned buffer. No new query lock, COW path, provider or capture framework is needed.**

## Existing capability and measured window

`vibe-safefs/src/file/bounded.rs:72–193` implements `Project::read_file_snapshot_bounded` and its pinned-directory form. It retains the project-root capability, locates a held parent, opens the final file without following links, checks regular/single-link ownership, takes two capped reads from that same file handle, compares their bytes, rechecks link count and length/mode, and reopens the final name through the held parent to compare object identity. It computes SHA-256 directly from the second buffer and returns that buffer, size, mode and opaque file identity together. There is no second path-based digest read.

`Project::open` (`project.rs:118–138`) opens a trusted absolute root through no-follow directory capabilities; `root_dir` clones the held root (`:146–154`). `Pinned::open_child` / `open_child_checked` (`project/pinned.rs:25–49`) open existing child directories without creating them. These read paths do not call lock, ensure-directory, publication or COW methods. Use those existing read seams, not `Project::lock` or `dir(..., true)`.

The snapshot holds file/parent capabilities during observation and final-name proof; the file handle is closed when it returns. The caller retains owned bytes and identity. That is sufficient for digest and parsing of the same captured bytes. C.1 does not require retaining an OS handle indefinitely after result construction. Its measured statement should be: two held-handle passes agreed, declared metadata/link policy held at checks, and the final name under the held parent matched that object at the final check. It does not prove uninterrupted immutability, timestamp stability beyond the checks actually present, or hostile ABA prevention. Opaque device/inode or volume/file-index identity explicitly has detection-bound semantics (`file/identity.rs:17–65`).

Do **not** use `read_linked_regular_snapshot_bounded` for the source-query route merely to make an installed hardlink pass. That is an explicit weaker artifact-output API. C.1 requires ordinary single-link ownership and truthful unavailable for unsupported shared source files.

## Concrete current consumer gap

`vibe-spec/src/resolver/read.rs:26–47` performs portable `checked_file`, then `read_stable_with` makes two independent `fs::read(path)` calls (`:63–76`). It compares bytes only. This accepted A.3 detection path is explicitly documented as **not** a retained-handle grant; C.1 is the next integration boundary.

A concrete same-bytes replacement can pass it: after the first ambient read, rename that source away and write a different regular object at the original path with identical bytes. The second read agrees, so current code returns success despite observing two objects. More sharply, replace the path with a symlink to a same-content file after preflight and before/after the first read: both ambient reads can follow that new path and agree. This is a source-level reachable counterexample, not an executed failure or hostile-ABA requirement. The existing SafeFS final-name identity check is designed to reject replacement **during its own held observation**.

The real consumer is `FsSectionSource::read_resolved` in `embed.rs:231–261` and its section path at `:286`: no-overlay Markdown reads call `read_projected_source`; custom formats use `read_preflighted_source`; projection calls `vibe_specdoc::project_spec_text` from the supplied byte buffer, then `resolve_section` parses that resulting text with `DocTree` (`:321`). Overlay bytes already form an explicit in-memory input; do not silently replace overlay semantics with an unrelated filesystem read.

Minimum integration: retain existing selected authority and portable lookup decisions, obtain a root-relative path, capture it through `Project::read_file_snapshot_bounded`, and carry a read-only observation/result that gives the **same snapshot bytes** to UTF-8/projection/parser and raw digest publication. Keep raw-byte digest distinct from projected Markdown or later semantic hashes. A.1's selected-world/legacy split and A.2/A.3 typed absence/unreadable/unstable failures must remain; disappearance after successful lookup is instability, not a newly fabricated stable absent result.

One deciding regression for this gap: exercise the actual new resolver/`FsSectionSource` capture seam, replace the selected final file with a distinct same-byte object **after the held file opens**, and require refusal; also assert the stable control returns bytes whose raw SHA-256 matches the buffer passed into projection/parsing. The existing SafeFS before-bounded-read hook is the concrete timing seam. Do not only test that two different contents differ: the old two-open implementation already detects that.

## Existing targeted proof owners

Inspected source tests, not fresh executed passes:

- `file::bounded_tests::stable_snapshot_binds_bytes_state_and_identity_to_one_epoch` checks actual snapshot bytes/size/identity and cap refusal.
- `a_file_renamed_under_the_read_refuses_rather_than_returning_either_object` injects rename/replacement after opening and checks final-name race; currently exercises `read_file_bounded`, so bind a snapshot/consumer sibling if the new integrated path needs its own exact proof.
- `a_symlink_at_the_final_name_still_refuses_under_a_cap`, `a_second_hard_link_still_refuses_under_a_cap`, and `a_hard_link_planted_under_the_read_refuses_at_the_reopen` preserve incumbent unsafe/shared-file refusal semantics. Their success cannot be claimed here without current matching execution evidence.
- Resolver `changing_bytes_between_reads_is_unstable_not_absent` characterizes A.3's old path; it is not proof of a held-object C.1 result.

No separate current digest-versus-parser mismatch is asserted: the old path publishes no raw digest in this helper and already parses its returned buffer. The concrete missing guarantee is the file-object/capability observation behind that buffer and a truthful result window, not a need to create another parser.

## Exact further defining owner requested

`file.rs:39–40` already exports `VerifiedStreamReader`, `StreamCompletion` and `StreamOutcome` from `stable::held_stream`. Those defining files were outside the initial packet, so Root was asked for **`crates/vibe-safefs/src/file/stable.rs`** and **`crates/vibe-safefs/src/file/stable/held_stream.rs`**, with **`crates/vibe-safefs/src/file/stream.rs`** only if its digest seam is needed. They were not read at this checkpoint. Do not add a duplicate held-stream API before inspecting that owner; the buffered snapshot path may already satisfy the bounded actual consumer.

Reviewed initial hashes:

- `file/bounded.rs`: `1791f235fddb62f78fcb61828fc826572d8d5aad15bb053b2a02f1dfc7555910`
- `resolver/read.rs`: `357a9f1245f7c97014bf70e307371a79d4321351393d9b9763c63bec4d1cf797`
- `resolver/portable.rs`: `54dce72d371063237eacafef8d509b248a1d147e01e63152d630c3805b313b9f`
- `project.rs`: `1adb308a571c857d10b3f446bb9810577c9b2531a3d7f4ae98f1838a07022f1d`

## Existing streaming owner checked after exact read grant

Root granted and this reviewer inspected `file/stable.rs`, `file/stable/held_stream.rs` and `file/stream.rs`. There is no reason to invent another held-stream abstraction for C.1:

- `Project::open_verified_stream_bounded` already opens a single-link no-follow file, proves two bounded hash passes against expected digest/size, rechecks its final name, and rewinds **the same handle** into `VerifiedStreamReader`. The reader retains its file and parent capability. At completion it compares delivered-byte digest/size, link/metadata/file identity and final name; early drop remains Uncertain. This API is designed for streaming already-expected bytes into an external body. It is not necessary for the actual buffered compiler/embed path.
- `Project::digest_file_in` streams two passes through one held object and performs final-name proof, but returns a digest without parser bytes. Calling it and then reopening the path for parsing would recreate the split-observation problem. Do not use the separate digest API that way.
- For `FsSectionSource`, keep the simpler `read_file_snapshot_bounded` composition: one bounded observation yields exact bytes/raw digest, then pure in-memory projection and section parsing consume those bytes. No extra lifetime or transport API is needed. Preserving the Project/root capability during capture and the immutable buffer through parsing suffices for this task's stated measured result lifetime.

The decisive identical-byte replacement case should run at the held-read boundary, not merely between lookup and the first open: a replacement before the observation begins can legitimately become the observed selected file if no earlier identity was promised. A replacement **after the held file opens** must fail its final-name check even when replacement bytes are identical. A link planted after portable preflight but before the SafeFS open must be refused by the no-follow open. These distinguish C.1's actual guarantees from broader atomic namespace or hostile-ABA claims. No tests or source/runtime edits were performed by this review.
