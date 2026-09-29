# Selected capability verification observations

Subject interval: capability-baseline-4a3f80a34 through
capability-after-2c35eed03. These are bounded GNU/Linux source, metadata and
declaration observations, not a full product/platform panel.

- Canonical generation: 106 schemas, exit 0. A later required registration-role
  field changed actual schema input and justified one further emission; only
  the Coverage leaf changed then. Other generated wrapper changes were reviewed
  as schema-directory comments, with the root export and FormatId changes separate.
- Owning wire family: 11 cases passed after the boolean-role change, including
  12 generated-reader fixtures. The exact authored-data case passed separately;
  after source/observation rebinding and the Coverage model addition it passed
  again on the changed data. No unrelated wire family was repeated.
- Native xtask helper family: three cases passed, including selected-case/zero
  execution refusal and the actual 30-node developer command table.
- Actual owning selectors: one CLI case, one MCP case and one TUI case passed.
  The final `cargo xtask capability-coverage` invocation with locked child builds
  exited 0 and observed exactly one successful case in each target. Metadata
  observation did not run operation handlers, providers or ServerContext.
- Existing VVM test repair: its exact Unix linked-mirror refusal case passed.
  Initial CLI compilation had failed before selected cases because unwrap_err
  accidentally required CloneOutcome: Debug; the test-only explicit match fixed it.
- Wire-library Clippy passed. Strict xtask Clippy failed in unchanged dependency
  sources; with no-deps it failed on unchanged vocabulary/dist lint debt. B-215
  retains those failures. No clean strict xtask verdict is claimed.
- Source index: 8,698 units, 3,834 tagged items and 3,255 edges; zero suspects,
  gated orphans and unresolved host edges. Thirty-five non-host edges remain
  outside that gate's jurisdiction.
- Capture through the DD source observation passed 30 source-byte/Git claims.
  The narrow later capture changed only the two published inventory data-file
  identities; package rows and three source-declared surfaces were unchanged.

The first offline CLI/MCP launches stopped before tests because pinned dependencies
were missing locally. Locked retries downloaded normal build dependencies and
left Cargo.lock unchanged. Existing compiler warnings remained visible.

No production mutation, workspace test suite, full native product gate, host
installation, public publication or prototype graduation was performed. Native
membership and source hashes do not authenticate a reviewer, establish target
authority or prove semantic/API completeness. Later owners retain those duties.
