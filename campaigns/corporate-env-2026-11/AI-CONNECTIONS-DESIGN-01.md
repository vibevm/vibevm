# Local AI connections: finite storage and binding design

Date2026-11-19. Architectural design for CORP-AI-CONNECTIONS/CORP-AI-SETUP, subordinate to CORP-AGENTS-01.md. Not implemented support. Owner selected the effective user settings root and multi-agent onboarding; Claude Code OpenAI compatibility is deferred. Implementation follows SYS-PROMPT/parity and earlier CORP stages.

## Owner amendment: generic local keyring with AI specialization

Owner clarification2026-11-19 supersedes the AI-only storage assumptions below: Vibe needs a general local keyring for multiple kinds of secrets, with AI connections as one specialization, analogous in purpose to a local password manager. The core root is `<effective user settings>/secrets/`; AI metadata/bindings remain under its `ai` namespace. Generic secret items have opaque identity, type/namespace, version and protected payload; AI connection revisions reference a generic secret item/revision rather than owning a separate raw credential file. Other consumers must use the same core, without depending on `vibe ai` or pretending every secret is an LLM key.

The generic core owns secure persistence, create/replace/remove/list-metadata operations, explicit consumer access, atomic revisions and recovery; AI owns endpoint/default model/protocol capability and agent/default bindings. Proposed CLI separation is `vibe secrets ...` for the core and `vibe ai ...` for AI configuration. No generic raw-secret stdout/show/export default is authorized. Existing never-print and no-arbitrary-hook boundaries remain binding.

Owner explicitly deferred encrypted vaults on 2026-11-19: the local machine is trusted for the current product. Implement a plaintext-at-rest local keyring with appropriate user-only filesystem protection, not encryption, master-password/unlock flows, OS-keystore prerequisites or cryptographic key management. Clearly document that local administrators, malware running as the user and backups may read plaintext; do not claim vault-level protection. Encrypted vaults are distant future scope and are not prerequisites of these stages. Never-print, no-follow/atomic writes, ownership checks and no automatic publication/inheritance still apply. The Bitwarden analogy means convenient local secret inventory/references, not cloud sync, hosted services, browser integration, sharing, automatic credential import or feature-complete password-manager replication.

Owner also requires a unique stable ID for every secret. Use an opaque generated UUID independent of its label, path, type or value; never derive it from the secret. Rotation retains logical ID and creates a new revision. A deleted ID is not reassigned to a new secret; unresolved references fail explicitly. Consumers store `secret_ref` IDs, optionally a separately admitted revision pin, rather than duplicate raw values. Labels can change without breaking references. An ID is a reference, not authorization to resolve a secret: user-local store and admitted consumer boundaries still govern access. AI connection metadata references this same generic core, as can other Vibe consumers.

The revision/index and default/override semantics below remain the AI specialization design, but physical `credential` files and the earlier 'not a general vault' exclusion are superseded. Final generic storage wire/layout and AI reference transaction must be settled together; do not implement the obsolete AI-only layout and migrate immediately afterward.

## Agent-facing secret references, not secret-valued tool results

Owner clarification2026-11-19: future agents must refer to these IDs while working without sending secret values to the LLM. SSH keys are the next anticipated type, explicitly NOT current implementation. Keep generic item/type/reference contracts capable of such an adapter without adding SSH import, signing, ssh-agent or forwarding now.

Separate the model-visible operation request from the trusted local executor. The agent supplies an opaque `secret_ref`, intended operation and destination; the local Vibe consumer resolves the value only at the admitted execution boundary. The model/tool result receives success/failure and safe metadata, never resolved credentials, headers, environment or file contents. Do not provide a generic model-callable `get_secret` returning plaintext or a general shell template that expands secret values into argv/stdout. An ID alone is not a bearer capability: check caller/session, operation, destination, consent and allowed consumer before resolution. Connection endpoint changes require revalidation so a prompt cannot redirect an existing key to an arbitrary host.

For an AI API key, local provider/native-agent adapter inserts it at the actual authentication boundary; it is never included in model messages, prompt context, system instructions or conversation history. Sending an API credential to its intended authentication endpoint is different from sending it as model content; do not promise that a credential used for remote authentication never traverses a network. Redirect/response/error handling and child output must not echo it into model-visible results. Use explicit permitted destinations and avoid making the local executor a confused deputy.

Trusted local-machine plaintext storage does not prevent a same-user unrestricted shell/read tool from reading its files. Therefore 'not exposed through managed operations' is the enforceable current promise; absolute model inaccessibility additionally requires actual filesystem/process isolation or an agent tool-access boundary. Document and test that distinction, never claim masking or IDs alone provide a sandbox. Existing worker isolation remains mandatory; local keyring paths are excluded from worker snapshots and generic context readers.

Future SSH adapter should prefer operation-based use (e.g. native ssh-agent signing/authentication) over exporting private-key material to the model. Agent forwarding and destination constraints are separate authority/security choices, not automatically enabled by storing a key. Preserve this extension point only; SSH functionality and encrypted vaults remain deferred.

## Complete terminal CRUD — interactive and noninteractive

Owner requirement2026-11-19: the generic keyring exposes complete create/read/update/delete in both interactive and noninteractive Vibe terminal surfaces, not just installer prompts or an internal API. Include list/search/select, item metadata/details, add, rename/edit metadata, replace value/rotate revision, inspect references, and reference-aware deletion. Read/details display the item and value presence with masking, never silently omit Read from CRUD. Secret-value access for admitted consumers remains supported through the core resolver; any explicit operator retrieval must use a deliberately selected protected output channel, not ordinary list/show/JSON/logs. Do not invent a raw-secret stdout exception to PROP-000 without a separate precise contract.

Interactive mode uses the existing Vibe terminal/question/display owners to provide a polished pseudographic form: boxed panels, clear title/help, keyboard-selectable item/actions, labeled fields, masked secret entry, validation near fields, reference impact preview and destructive confirmation. Claude Code AskUserQuestion is a usability reference, not a requirement to clone its assets or implement a second UI framework. Provide predictable Tab/Shift-Tab/arrows/Enter/Escape behavior, resize handling, readable narrow-terminal/plain fallback and terminal restoration on cancel/error. Do not echo a previously stored secret into an editable visible field; replacement is explicit and blank input does not accidentally clear it.

Noninteractive commands offer the same operations and atomic results through stable arguments plus redacted machine-readable output. Values arrive by protected stdin/file/descriptor conventions rather than argv; distinguish missing input from an empty replacement and never prompt or hang without a TTY. Update/delete support exact item ID, expected revision/preflight and actionable conflict errors. Cancellation, validation failure, referenced deletion refusal and interruption preserve working store state. User-facing outcomes and underlying operation semantics must match the interactive form, CLI and AI onboarding, with one CRUD planner/executor rather than independent stores or business logic.

Finite acceptance includes real terminal create/select/details/edit/rotate/delete, keyboard and cancellation flows, noninteractive CRUD/JSON and protected input, reference-aware deletion, resize/plain fallback, exact secret-redaction sentinels and terminal cleanup. This is a terminal feature, not a new graphical password-manager application.

## One store, explicit bindings

Root: `<effective user settings>/secrets/ai`. Resolve through the existing user-settings selector, never cwd/project/application install root. Fail if the selected root cannot provide required user-only security; do not fall back to a different store. A corporate profile supplies defaults/hints only, never connection secrets or bindings that override personal choices.

Store logical connection IDs separately from immutable secret-bearing revisions:

- `index.toml`: schema, store revision, optional default connection ID, connection-ID → revision reference, harness → binding mode.
- `connections/<opaque-id>/<opaque-revision>/config.toml`: schema, `api_kind = "openai-chat-completions"`, endpoint and required `default_model` for a usable configured connection (owner clarification2026-11-19). No embedded credential URL, unknown provider inference or raw key.
- Generic `items/<secret-id>/<secret-revision>/value`: the user-protected plaintext value, resolved only by the generic keyring. AI connection config stores `secret_ref` and an admitted revision/reference policy; it never owns a second credential file.

All files remain within the protected root, including metadata. Names are generated opaque IDs, not arbitrary user paths. Human labels are metadata and do not decide path/authority. AI metadata and its referenced generic secret revision are validated and selected as one logical connection update; neither physical co-location nor duplicated secret bytes are required. Default references a logical ID; updating that connection changes its selected revision. No duplicate `default` secret copy.

Initial index semantics (illustrative TOML, not a registered format yet):

```toml
schema = 1
revision = 1
default_connection = "opaque-connection-id"

[connections."opaque-connection-id"]
revision = "opaque-revision-id"
label = "Default AI"

[agents.pi]
mode = "inherit-default"

[agents.hermes]
mode = "connection"
connection = "another-opaque-id"

[agents.claude]
mode = "native"
```

Missing agent binding means native/unmanaged, not automatic opt-in or guessed inheritance. The setup wizard explicitly creates inherit-default bindings for selected compatible agents that consented. This keeps adding a default from unexpectedly replacing every existing agent's auth. Aliases resolve to harness identity before lookup; the user's executable override remains intact.

## Atomic update and recovery

Acquire the existing applicable user-scoped mutation lock or a narrowly owned AI-store lock, with consistent ordering. Admit paths with no-follow capabilities and user-only permissions/ACL before writing any secret. Validate protocol/endpoint/key shape without a network test. A connection check requiring a request is separately consented, not an automatic wizard effect.

For add/update, write a new revision config (endpoint/API kind/default model) and credential with exclusive creation, then atomically publish one validated index selecting that revision. The wizard treats endpoint, key and default model as one reusable connection; status shows endpoint/model and credential presence only. Explicit per-invocation model choices override this default through native owners, and individual agent model overrides are not erased by a default update. Never mutate an active revision in place. Failure before index publication leaves the old working selection; failure after publication cannot expose a mismatched endpoint/key pair. Record no secret in recovery journal. Refuse foreign existing paths and symlink/reparse aliases rather than repairing them.

Readers load one index snapshot and its admitted revision; retention must keep that revision available until the reader has copied the required values. Do not race launch with eager deletion. Replacement makes old revisions unreachable but does not promise secure erasure; a bounded cleanup owner can remove old revisions only when no active reader needs them. Do not let general application cleanup traverse this store. Uninstalling/updating an agent or Vibe must not silently erase the credential inventory.

Removal first resolves all references and asks for an explicit replacement/native/clear disposition, then publishes one index transaction. Never delete a default or per-agent connection and silently redirect to another credential. Cleanup of unreachable files follows the same safety boundary. Cancel leaves index, connections and native agent stores unchanged.

## Resolution and replacement semantics

At Vibe-managed launch: resolve launcher alias → harness → explicit binding → logical connection → immutable revision → protocol capability. An unavailable default, removed connection or unsupported API is an actionable error for a managed binding, not fallback to ambient credentials. Native/unmanaged mode leaves ordinary agent authentication alone.

Changing only the default reference affects inherit-default agents on the next launch; explicit connections/native agents stay unchanged. Editing a shared connection affects all explicit references to that logical ID, so update confirms the affected bindings. 'Replace all' is a separate transaction rebinding only displayed Vibe-managed compatible agents after explicit confirmation; it never edits arbitrary native OAuth/config stores. An endpoint/key replacement clearly names whether it edits a shared connection or creates a new one. Easy per-agent override creates/selects another connection without rerunning installer.

Claude Code remains native/unmanaged for this initial OpenAI-only setup; it can be installed and proxied, but receives a compatibility warning instead of an OpenAI key prompt. API capability is an adapter declaration backed by actual native evidence, not a product-name heuristic. No protocol translation layer is introduced here.

## Security and finite gates

Redacted list/show/JSON report only label, safe endpoint, API kind, presence and binding source. Secret input uses masked TTY or a protected input channel; no key-valued argv, shell history, environment dump, serialized installer profile, shared manifest, lockfile, drop or diagnostic. Native delivery must be scoped to the selected agent invocation using an explicitly admitted mechanism; arbitrary hooks/children inherit no grant. Apply the narrow PROP-000 amendment described in CORP-AGENTS-01 before implementation.

Required tests: owner-only Unix and Windows storage admission; no-follow/race/foreign paths; endpoint/key consistency across interrupted index publication; default versus explicit/native bindings; alias resolution; referenced deletion; cancellation and multi-agent partial/deferred outcomes; secret redaction across error/JSON/debug; live-reader versus retirement; native credentials untouched; Claude API setup skipped. Use synthetic keys only. This design is a generic local plaintext keyring with an AI specialization, not an encrypted vault, remote secret service, automatic credential importer or unbounded key-management project.
