# Windows private storage — test-only normalized policy

## Scope and implementation boundary

The original CORP-AI-CONNECTIONS criterion requires secure user-local storage on Windows, including explicit owner/DACL and reparse admission. Current `vibe-safefs::private` refuses non-Unix private access. The private stage creation and exact-mode publication paths also refuse rather than treating missing Unix modes as confidentiality. These refusals remain unchanged.

The architect authorized one executable predecessor: a pure policy classifier over normalized synthetic descriptor facts, limited to private objects and compiled only under `cfg(test)`. This is not a raw binary security-descriptor parser, SID parser, AccessCheck replacement or held-handle observation. Rights are semantic labels, not Win32 numeric masks. A future normalizer must independently prove canonical SID identity and translate actual descriptor/ACE/access-mask forms without dropping unknown bits or flags.

The classifier requires current normalized SID ownership, protected supported descriptor controls, present non-NULL nonempty DACL and explicit basic allow entries for the current SID only. Every nonzero ACE flag is refused, including inheritance and propagation. Foreign trustees, unsupported ACE forms and unknown rights fail closed. File and directory profiles have distinct admitted and required rights; grants may be split across admitted ACEs. This narrow candidate does not implement the separate readable-parent policy or infer that policy from private-file admission.

## Verification status

Evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z/`. `private-acl-policy-build-r1` compiled the SafeFS library test artifact without executing it, exit0. `private-acl-policy-runtime-r1` then executed the actual artifact in credential-free filesystem/PID/network isolation on Linux: **5/5 policy tests PASS**, 165 unrelated tests filtered, no ignored tests. Tables cover both positive object profiles, split grants, foreign/empty owner, unsupported descriptor controls, absent/NULL/empty DACL, unsupported ACE, foreign broad trustees, every one of the 255 nonzero ACE flags, unknown/empty/insufficient/cross-profile rights and admitted optional rights. `private-acl-policy-strict-r1` completed all-target SafeFS Clippy with warnings denied, exit0.

There is no native Windows runtime claim, no Windows cross-compilation claim and no integration into secret storage or publication. No host policies, CI, system components or private user state were changed.

## Required native gate — still open, not waived

Before any Windows private-write branch can be enabled, a qualified isolated native executor must demonstrate current-token identity and owner/DACL classification obtained from the same retained no-follow handles used for I/O; secure protected descriptor installation at object creation before secret bytes; safe directory/lock/stage birth; reparse and hardlink refusal; collision/substitution and inherited/broad/unknown-grant negatives; and final publication/lock identity and descriptor rechecks. Private birth and real handle acquisition are separate from this classifier. Actual public cold setup remains required after primitive qualification.

No such isolated Windows test entrypoint is currently verified. Ordinary-profile execution on the available Windows host is not an authorized substitute. macOS Intel release/installer/cold setup and foreign Hermes archive builds remain separate open conditions; Linux acceptance does not promote them.
