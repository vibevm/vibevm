# Explicit manifest migration in mutable 1.1.0

This local next implementation applies an approved migration to one owned
authored manifest. Select the file yourself; fixtures, upstream packages and
historical captures are not migration targets. Nothing is migrated on startup.
The product and owned package version remains mutable 1.1.0. This document
does not authorize public publication.

## Plan, review, then apply

Use `vibe refactor plan-migration` with a JSON request on stdin, or its
`--request-json` argument. For the recognized legacy materialization spelling,
the request fields are:

```json
{
  "schema": 1,
  "source_text": "the exact UTF-8 contents of your manifest",
  "source_profile": "materialization-legacy-absent-r1",
  "target_profile": "materialization-current-absent-r1",
  "recipe": "snapshot-to-copy-r1"
}
```

The example's source text is a placeholder. Supply the actual document, review
the returned before/after text and diff, and proceed only with a `planned` or
`no_op` result. A question or refusal needs resolution first. Source and target
profiles describe authored format semantics; they are independent of product
version 1.1.0.

This recipe handles recognized package-role `materialization = "snapshot"`
to `"copy"` syntax. The current-format identity recipe preserves an admitted
current document. These finite recipes do not convert arbitrary project
manifests or establish compatibility with the complete historical corpus.

Build an application request from that reviewed plan. Keep the original
planning request as the JSON string `approved_request_json`; copy the plan's
`before_identity`, `after_identity` and `diff_identity` into the application
fields below. Do not compute substitute hashes or supply unchecked after text.

```json
{
  "schema": 1,
  "operation": "apply",
  "operation_id": "my-reviewed-migration-1",
  "authority": {
    "project_root": "/absolute/project",
    "target_relative": "vibe.toml",
    "recovery_store_root": "/absolute/separate-recovery-store",
    "scope": "owned_authored_file"
  },
  "approved_request_json": "the original planning request encoded as a JSON string",
  "before_content_id": "copy before_identity from the plan",
  "after_content_id": "copy after_identity from the plan",
  "diff_id": "copy diff_identity from the plan"
}
```

Replace every placeholder before invoking
`vibe refactor migration-operation < apply-request.json`. The explicit CLI
invocation grants this single selected file. Project/store paths must be
absolute; on Windows use absolute Windows paths encoded as JSON strings.
The target uses portable relative components. The recovery store must be
physically separate from the project. Keep the same project spelling, store,
operation ID and exact approval on replay. Reusing an ID for a different
approval is refused.

Validated normal results emit one JSON report and a newline. Exit 0 means completed or
no-op; exit 2 means pending inspection or malformed/unsupported input; exit 1
means refusal, uncertain effects, input I/O failure or an internal failure.
Read `effects`, `warnings`, `residue`, `target_state` and `guarantees` even on
failure. An error can follow a completed or uncertain write.

## Inspect or recover after interruption

Send an `inspect` request with the same schema, operation ID and authority,
omitting the four apply-only fields. Inspection opens existing evidence and
creates no new intent or target write. Missing evidence is a refusal.

For recovery, set `operation` to `recover`, retain that authority and ID, add
`expected_intent_id` from the inspected report and set `action` to `complete`
or `restore`. Completion continues a supported publication prefix; restoration
continues a supported restore prefix or cancels an unpublished intent. Unknown,
missing or unreadable target states require explicit resolution. A terminal
restored operation is not silently reapplied. Exact terminal replay makes no
new target write.

Keep retained `before.bin`, `journal.json` and reported residue until the
operation is resolved. A stage filename alone does not prove ownership or
authorize deletion. The implementation does not automatically clean unknown
occupants. Internal report-invariant failures print clearly labelled
`UNVALIDATED` diagnostic evidence; that evidence is not a normal report or
replay authority.

## MCP admission

The tool is `manifest_migration_operation`, with the same request protocol.
The server must be explicitly started with `vibe mcp serve --path PROJECT
--migration-recovery-store STORE`. With no `--migration-file` flags it grants
only `vibe.toml`; repeated `--migration-file RELATIVE_FILE` flags specify a
finite allowlist instead. The server uses its resolved project and excludes
its protected machine cache from recovery storage. A payload path or scope
label cannot expand the server grant. A server started without the recovery
store flag refuses application admission.

The MCP transport distinguishes preflight from executed failures through its
response channel and message; the operation report has no separate typed
preflight/executed discriminator. Read its effects even when `isError` is true.
Internal invariant failures retain diagnostic text and omit normal
`structuredContent`. This distinction prevents clients from treating invalid
evidence as a validated successful response.

## Guarantees and platform evidence

Linux, macOS and Windows have implementation paths. The current measured
receipts cover selected GNU/Linux tests; native macOS and Windows execution
has not yet been measured. API availability and source review are not native
test results.

Participating clients coordinate through the same physical recovery store and
project identity lock. Different stores, ancestor/descendant project scopes,
uncooperating writers, already-open external file descriptors and undetected
ABA changes are outside this guarantee. Exact bytes and identity are checked
before publication and the resulting file is verified afterward, but ordinary
filesystem replacement is not unconditional content-CAS. The report always
sets `uncooperating_writers_excluded` to false.

Preservation covers exact bytes and Unix permission mode where available.
Windows reports bytes-only preservation. ACLs, ownership, extended attributes
and timestamps are outside this preservation scope. Reduced metadata and
durability guarantees produce warnings.

Successful file-data synchronization is required before a target write.
Namespace synchronization can yield `journal_recoverable` where the platform
cannot confirm directory persistence; this does not prove survival of power
or system failure. Process recovery requires surviving retained evidence.
Genuine synchronization errors may refuse and report uncertainty. Neither
missing unrestricted CAS nor a weaker namespace tier disables an entire OS.

The bounded source/evidence account is
[`migration-apply-transition.json`](migration-apply-transition.json).
Final preview, complete API/documentation coverage and native platform proof
remain separate campaign obligations.
