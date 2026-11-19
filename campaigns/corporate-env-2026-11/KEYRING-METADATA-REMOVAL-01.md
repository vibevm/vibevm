# Keyring metadata edits and reference-aware removal

The accepted Unix storage baseline is `2a5ad585f`. This slice works only on injected temporary settings roots. No real credentials, OS keystore, Windows ACL claim, AI-specific storage or raw-value accessor.

Expose a redacted metadata snapshot with both store revision and current item revision. Under the same protected store lock, require both expected tokens before a label edit or logical removal. A label edit does not rotate the value or change the stable ID; its successful publication advances the store revision once. An unchanged label has no write effect. A stale request returns a conflict without changing the index. Kind changes are outside this operation.

Removal asks the registered consumer owners for references, never scans arbitrary user files. A known reference blocks removal and returns only safe reference identifiers; missing, inconsistent or unverifiable reference information also blocks removal. An explicitly empty participating consumer set is distinct from an absent callback. Registration writers must participate in the same serialization boundary; never infer that a stale scan is current. No implicit reassignment or force flag.

On successful removal publish an index without the item. Keep its private ID directory reserved and do not eagerly delete immutable revision bytes. New details lookups must return not found; a second remove is not a second mutation. This is logical removal, not secure erasure. Preserve foreign neighbors and read-only lookup behavior.

Acceptance uses synthetic fixtures: edit versus rotate with stale tokens, no-op edit, referenced/unknown/unreferenced removal, repeated removal, interruption before publication, retained old revisions and non-reuse of an orphan ID. Compare index bytes before and after refusals. No output or error may contain a value sentinel.
