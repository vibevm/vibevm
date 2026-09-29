# Selected architecture source verification

GNU/Linux observations for the selected interval:

- Six changed XML sources passed the canonical vibe-specdoc reader. Only the
  two actually changed revision/rationale sources were re-read after follow-up.
- Exactly one existing owning case ran with the bound selector
  `cargo test --locked --offline -p vibe-mcp --lib agentic::tests::park_then_drain_is_a_one_shot_mailbox -- --exact`:
  one passed, zero failed, 66 filtered. Its body is unchanged; it proves mailbox
  lifecycle against the unchanged directory contract, not new role admission.
- Final canonical index: 8,712 units, 3,835 tagged items, 3,256 edges; zero
  suspects, gated orphans and unresolved host edges. Thirty-four warnings and
  35 non-host edges outside jurisdiction remain. Earlier intermediate output
  exposed one old broad revision pin; the precise citation resolved that debt.
- Independent seven-source review and final relay follow-up found no remaining
  source-policy blocker. Two extra read-only baseline queries exceeded its
  packet and were disclosed; they changed no source or gate state and are not
  used as proof of new runtime behavior.
- Before and after capture authoring exited 0: both retain 3,699 selected files,
  48 packages at 1.1.0 and four unchanged source-declared surfaces. After raw SHA
  is dee618cbc66b0d2dec35dd6c1aeb16cd18dbdb39681a7357ea397b9d6df97e92;
  normalized SHA is f759ae5f05eb757144fc650703d4f8b3463b3e44f2ae8c11470f1552aecfea1f.
- Git whitespace checks passed. No Rust implementation or schema changed;
  no global formatting, wire/codegen, workspace test or product panel ran.

No production mutation or public operation occurred. Proposed role guards,
source classification, calculator and universal API conformance remain unrun
implementation obligations. Captures and source references do not grant effect
authority or establish complete documentation/native linkage.
