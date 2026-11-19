# Pi — first bounded corporate-proxy journey

2026-11-20. Architect independently matched final2 manifest/source fingerprints and accepted the bounded slice. Product commit `9fdc51907`; final2 manifest SHA-256 `508d3fdb921d214ad308d45d9f9b36d309f6c16c74ce20e49a8caa724e2c0004`, tested Vibe binary SHA-256 `d4de77d8109f71ed01218d6429823fc85a90ae0d6c201730c430fa141eaef717`. This slice is Linux Pi 1.5.2, not six-agent/platform parity or CORP completion.

## Shared owners

The existing tool-px lending owner projects the configured invocation route into the already prepared child command. Pi's argv prefix, standing arguments, cwd and carrier preparation are not reconstructed by another launcher. No configured route retains the original Unix exec behavior. An external proxy overlays only child environment; a Vibe-managed px retains its NetworkSession while the child runs, then closes owned px on normal exit or start failure.

The existing PATH-proxy Supervisor is promoted to one shared source. It consumes its own terminal INT/QUIT copies without forwarding them again, forwards pending TERM to the child and preserves the child status. Existing narrow FFI annotations remain; no module-wide unsafe allowance or second supervisor is introduced. Directed parent INT, exact-once group TERM, detached descendant lease lifetime and other OSes are not qualified here.

With a configured corporate route, other unqualified harness identities refuse before native effects rather than silently launching unproxied. Ordinary no-route behavior stays unchanged. Hermes READY attachment remains local-only and does not acquire or change a separately owned inference route.

## Native proof and limits

Credential-free bubblewrap fixture: isolated filesystem/process/network namespaces, cleared environment, synthetic HOME/settings, read-only pinned Pi runtime/Node/control/binary/system mounts and owned writable evidence. No host auth/profile, remote provider or model charge. The local provider implements deterministic OpenAI-compatible SSE; Pi uses its own ordinary provider/client/dispatcher, not a replacement fetch or extension that merely records a pre-transport request.

The fixture runs outside a Vibe project. It proves the selected proxy route, **not simultaneous injected project boot delivery**. Bare Pi sends the same sentinel input to a loopback provider; Vibe-launched Pi targets a non-resolving non-loopback test hostname and succeeds only through the selected proxy's CONNECT mapping. Exactly one main request is added by the proxied arm, and the provider returns the expected sentinel answer. No live remote inference occurs.

Controls: managed px startup failure exits nonzero with no additional provider request/no direct fallback; bare native Pi remains unaffected by Vibe's route configuration; Vibe Pi loopback target intentionally bypasses the proxy. The four successful submissions across baseline/proxy/bare/bypass are four fixture requests, not four independent platform qualifications. Proxy-side log witnesses the selected non-loopback CONNECT. Managed px process IDs are gone after each terminal outcome.

Controlling PTY proof verifies foreground process-group identity before sending terminal Ctrl+C: child observes one SIGINT and deliberately exits 43. TERM directed at the Vibe wrapper is forwarded; child deliberately exits 42. Normal child exit 7 and spawn failure exit 1 are preserved, and owned px is cleaned in all four cases. These are bounded shared-supervisor tests with a synthetic child, distinct from the native Pi provider proof.

## Verification

Persistent `cli_agent_proxy`: selected-route child environment, exact spaced argument, child status, spawn-failure cleanup and unqualified-harness refusal. Existing `cli_tool_px`: 8 PASS; unchanged PATH-proxy signal integration: 1 PASS; promoted supervisor test: 1 PASS. Strict affected clippy and format/diff gates are recorded in the private evidence. Final-source reruns and exact fingerprint manifest determine acceptance; an earlier green binary is not silently relabelled final.

Final2 native and four signal reruns passed on that exact binary; strict clippy, persistent route guard, no-route soft launch and original proxy-supervisor regressions passed. Private root `/data/git/vibevm-corp-pi-proxy-20261120T0001Z/` retains native request/response steps, provider requests, proxy CONNECT log, px lifetime, PTY signal results and source/binary fingerprints. Initial REDs are preserved: mounting an individual helper under an already read-only control mount, and a native baseline waiting for stdin EOF. Corrected fixture closes stdin, uses the exact user launcher table and separate immutable helper mount. Initial needless-return lint was corrected without suppression.

Remaining: additional native agents and platforms, combined project-carrier/proxy proof where required, explicit external-route native qualification, controlled cancellation cuts and detached-child ownership if claimed. No public publication or owner-attended check follows automatically.
