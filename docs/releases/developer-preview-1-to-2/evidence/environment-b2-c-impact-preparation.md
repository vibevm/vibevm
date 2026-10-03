# M-14 B.2/C release-impact preparation — candidate only

The following 23 subjects complete the [33-subject inventory](environment-bc-product-subjects.json)
after the ten B.1 entries. This worksheet records a candidate effect and
documentation topic from the accepted [B.2](environment-b2/central-review.md),
[C.1](environment-c1/central-review.md), [C.2](environment-c2/central-review.md)
and [C.3](environment-c3/central-review.md) reviews. It does not decide a
permanent `release-impact.json` record, migration action or no-impact claim.
The B/C capture candidates remain local pending B-226 (capture evidence
contract/import choice).

| Slice | Subject | Observed contribution | Final-review question / documentation topic |
|---|---|---|---|
| B.2 | `99e048e3b` | PROP-076 legacy evidence/source substitution law | Explain historical proof strength and incomplete recursive closure without upgrading old records. |
| B.2 | `1dc5a7097` | Pure legacy package classification and substitute/prefix policy | Document typed missing-input decisions; inspect actual consumers before claiming a user migration. |
| B.2 | `d04a93d74` | Validated artifact reader retains full record beside weaker legacy view | Distinguish recorded `verified` from current output-byte verification; name the reader API. |
| C.1 | `0ef17ab55` | PROP-076 opt-in bound operation law | Explain all-or-none binding and the shape-only effect refusal. |
| C.1 | `b80b3ce83` | PROP-076 bound journal/state identity | Document separate bound namespace and refusal to reconstruct copied legacy sidecars. |
| C.1 | `8f072d332` | PROP-076 explicit target mode | Describe why target mode cannot be inferred from a root or request shape. |
| C.1 | `d4d76a259` | Epoch-one environment binding in wire and deploy carriers | Identify the registered fields, nullable/required shape and legacy absent-binding compatibility. |
| C.1 | `b82f245ea` | Wire validator requires explicit mode | Compare before/after decoder refusal and decide whether this correction joins the binding format record. |
| C.1 | `15e4df996` | Bound state key, replay identity and structured inspection | Document typed early refusal; do not claim a successful bound effect from C.1. |
| C.2 | `592241b72` | SafeFS lock within a pinned destination | Explain cooperating-writer lock scope and its hostile-writer limit. |
| C.2 | `50ccf4144` | Retained pinned descendant project | Name the held-root API and same-root alias constraints without claiming content-CAS. |
| C.2 | `7ce63a97d` | Prepared pinned vibe-bin store operations | Document physical settings/bin/store ownership as a preparatory seam, not a published effect. |
| C.2 | `09bd6fe74` | Lock and pin named bound resource children | Distinguish alias contention from unrelated settings roots. |
| C.2 | `681b5f19f` | Bound state opened from pinned settings | Keep legacy state separate; review exact source/API visibility before a user-impact disposition. |
| C.2 | `2965bdd59` | Host-issued local target admission | Explain fresh generation/principal checks and why the public shape-only path still refuses. |
| C.2 | `93859cdf1` | Generated bound deploy sidecar/carrier fields | Name wire compatibility and exact target binding across pending/receipt/inverse carriers. |
| C.2 | `d0b458c53` | Prepared bound provider and durable carriers | Classify preparatory provider API versus admitted effect; inspect failure/partial truth. |
| C.2 | `397f11456` | Corrected generation and named-child rechecks | Explain stale generation and retargeting refusal; decide whether it joins the physical-fence record. |
| C.3 | `4bfdf7fb2` | First internal host-admitted local vibe-bin apply | Document first-generation UserProfile builtin scope, two-root outcome and no public CLI/MCP grant. |
| C.3 | `7165cd7b8` | Receipt listing through held bound state | State why foreign/copy receipts cannot be adopted; inspect exact user-visible refusal path. |
| C.3 | `3b31a6e67` | Admitted bound provider publication | Explain payload/launcher/pointer order and bounded `reversible=false` guarantee. |
| C.3 | `d643e2926` | Bound transaction journal and post-write uncertainty | Distinguish durable intent from verified receipt and cleanup uncertainty; no automatic interrupted recovery. |
| C source | `59812ce39` | PROP-076 portable physical fence law | Document the Linux-tested cooperating-writer guarantee and pending native macOS/Windows proof. |

For all B.2/C subjects, no automatic 1.0.x user-state migration or public
bound command was accepted by the source reviews. That is a group-level
boundary, not a per-commit no-user-impact verdict. Positive documentation
topics remain due for legacy-record strength, source substitution, opt-in
binding, physical alias fencing, first local builtin effect and uncertain
post-write truth. The original C.1/C.2/C.3 checks and the later selected
Linux gate receipt prove bounded behavior; neither substitutes for before/
after capture review, native foreign-platform execution or the 23 permanent
impact dispositions.
