# PP-002 — DEF-C2-2b-full: acceptance-backed worker credibility on the boss surface

_Filed: 2026-07-10 · Status: **POSTPONED** (needs its own mandate —
own slice or Campaign 3) · Origin: Campaign 2 close — plan §15
(DEF-C2-2 b), the Ф6 falsifier analysis (F24), WAL §Next._

## The task

Make worker capability **provable on the boss surface**, instead of
asserted.

Ф6's strongest observed lever on cold delegation was the boss's
*belief* that workers can self-verify: with the staging toolchain
broken, bosses in BOTH arms hand-fixed the linker and rationally
kept all work ("workers can't self-verify here"). DEF-C2-2a fixed
the toolchain itself; DEF-C2-3 shipped the thin slice — the cold
board's honest, static "fabric ready" line. **2b-full is the
credibility half:**

- **MC grows acceptance-schema plumbing:** worker runs record
  acceptance outcomes (e.g. "ran `cargo test`: exit 0, N passed")
  into the one telemetry store (I3 — no second store).
- **Boss surfaces cite them as dated facts:** scoreboard / cold
  board / mid-work nudge lines of the shape *"workers run cargo
  test green here, last proven \<when\>"* — evidence with recency,
  not assertion.
- **D7 binds:** the scoreboard is strictly factual — a credibility
  line exists only when a recorded acceptance fact backs it, and it
  carries its age.

## Why postponed

Needs acceptance-schema plumbing MC does not have (schema design,
packet/acceptance recording, store and surface changes) — too large
for the same-day DEF-C2 slice at the Campaign 2 close; deliberately
deferred there (plan §15, disposition "(b) OPEN (next campaign)").

## Unblock

The owner commissions it — as its own slice or folded into
Campaign 3's mandate. Design questions to settle at commissioning:

- the acceptance-schema shape (per-packet acceptance criteria vs
  post-hoc verification runs);
- the staleness rule for "last proven \<when\>" (when does a proof
  expire?);
- which surfaces cite the fact (cold board / scoreboard / mid-work
  nudge / statusline).

## Interaction with PP-001

Independent — MT-C2-05 can fire before or after this lands. If
2b-full lands first, MT-C2-05's arm definitions need a note (B′
would then carry the three DEF repairs *plus* credibility facts,
changing what the B′−A′ delta measures).

## Canonical pointers

- Deferral record + levers analysis: plan §15 (DEF-C2-2) and §2
  ("the two strongest observed levers") —
  [`FRACTALITY-INITIATIVE-PLAN-v0.1.md`](../../fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md)
- D7 — the strictly-factual scoreboard constraint: plan §6 D7.
- The thin slice that shipped instead (cold-start board):
  [`reports/2026-10-07-21-30-defc2slice-report.md`](../../reports/2026-10-07-21-30-defc2slice-report.md)
