# Need 2 acceptance audit

**Verdict: fully met in the qualified Linux/source scope at accepted commit 2df122323ce85fec5eae3533c34f21ce51b6cb7c. No actual unmet entry2 obligation identified.** Root retains acceptance. This verdict does not complete entry1 or entry3.

| Exact entry2 obligation | Verdict | Evidence |
|---|---|---|
| Global inventory retains registry, URL, ref, resolved commit and kind | Met | Shared SourceEvidence captures known resolver/acquisition facts; apply copies it into durable Installation. Unobserved optional facts stay absent. |
| app list/describe return these facts in JSON | Met | list.source_evidence is durable history. describe.installed_source returns installed history separately from current authored source_evidence. Final CLI verifies both. |
| Old installations report unknown, never guessed | Met | Old records promote without evidence. Final CLI changes current registry settings and verifies installed unknown remains null; pending legacy steps remain unknown after resume. |
| --plan-only --global names the source | Met | Existing global plan emits shared source_evidence, including known registry/URL/ref/commit/kind. Final CLI proves local/global alias and common-configuration projection. |

Entry2 expressly lists the five inventory facts above. Its “names the source too, as in entry1” cross-reference does not independently require implementing entry1's project no-write preview, real-install event, pins, candidate reasons or hosting preference. Hosting appears conditionally in entry1 (“if vibe knows it”); entry2 does not add a hosting-classification obligation. Treating missing standalone hosting classification or the separate project-event follow-up as an unmet entry2 requirement would import extra scope. Entry1/3 still require their own original behaviors; this finding does not weaken them.

Describe retains authored-package acquisition behavior; installed list is the inventory-only route. Entry2 requires facts returned, not a new offline/no-acquisition describe contract. Likewise global plan's established acquisition/cache effects do not satisfy the separate no-write project preview, and do not negate its ability to name the global application's source.

Observed receipts: final source CLI6/6 (cli-embedded-refresh.log, exit0 in embedded-gates.json); application221/221 (app-final-02.log, exit0 in final-gates-02.json). They cover captured/current distinction, legacy unknown, pending resume, fresh portable import versus skip, update/rollback and configured surfaces. No reviewer tests were run. Earlier fixture failures remain separately preserved in the prior retention review; none is relabeled as pass.

Accepted blobs:24 of26 reviewed hashes match the prior frozen review. The other two are the documented full_store validate specification annotation restoration (no executable change) and root-owned PROP-073 specification update. Accepted hashes, original requirement section hashes and exact receipt hashes are in the sibling JSON. Linux controls do not claim native Windows/macOS execution. The feature document's broad “not acceptance of 1–3” wording is a workflow/status disclaimer, not an additional acceptance criterion for entry2.
