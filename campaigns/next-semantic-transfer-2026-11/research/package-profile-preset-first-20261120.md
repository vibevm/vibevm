# Selected profile boolean preset — implementation candidate

Accepted qualification `b12e6fd94`; implementation contract integrated `f11aa7280`. FMT-085 records bounded semantics. No new persisted field, epoch, policy store or update renewal.

One required public bool without declaration default may be supplied by the explicitly selected profile. Admission reads the existing normalizer's typed parameters: selected preset first, then explicit consumer override, including false. Cross-consumer agreement uses this same effective result; original consumer omission/equal/opposite input remains untouched. Read-only projection validates the complete retained record, requires a real consumer and invokes the same normalizer rather than implementing independent precedence. An unselected profile supplies nothing. Private/defaulted/forwarded parameter modes remain closed.

Retained historical profile definition and original consumer maps suffice for offline replay and distinguish reliance from explicit answers. Existing profile/parameter semantic guards still refuse changed selected preset during update; explicit matching and opposite controls remain explicit rather than being rewritten into reliance. No consent transport or broader update support is added.

## Evidence boundary

Private root `/data/git/vibevm-keyring-storage-20261120T0100Z`. Build and scoped core/CLI strict lint passed in r1. `profile-preset-first-full-r1.log` records all57 public tests PASS on executable SHA-256 `83c0cf88edbea2fafc62437cb8b664ff6a29ffce3796254f755f0de3279796c6`, fixture SHA-256 `d3ed166bf87d7457e7879ea0ed84474141acca105a42f9ba8c741df6b50e2765`. Actual prior fac5fe80 reader and install writer refuse preset-bearing records with state unchanged. Formatting/diff checks passed. Gates cover true preset omitted/equal/opposite inputs, false preset overrides, no-preset controls, actual workspace agreement/conflict, unselected preset refusal, actual prior reader/install writer unchanged refusal and preset drift refusal before lock/slots/boot/hooks. Superseded qualification assertions change only for this newly supported behavior; historical refusal receipt remains evidence of the earlier runtime.

No agent/provider/network, arbitrary profile parameters, general defaults/forwarding expansion, USER-PROJECT adoption or NST parent completion. UP adoption remains UNRUN; REMAINDER/AR/IDE unchanged. Exact source/binary manifest and independent review are required before commit.
