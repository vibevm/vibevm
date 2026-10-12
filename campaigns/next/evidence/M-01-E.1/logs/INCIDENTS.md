# M-01-E.1 incidents

- Initial representative command failed in shell redirection (exit 1): evidence directory did not exist. Cargo did not execute; no test verdict. Directory created before retry.
- Initial batched source reads exceeded output limits. Read remaining defining-owner sections separately; truncated output is not complete evidence.
- Initial exact representative selector omitted `read` module: exit 0, 0 tests, 995 filtered. Not counted. Corrected literal registration path and reran.
- spec-e1-first.log: compile exit 101, E0277 at two fixture assignments: I assigned a string to existing Vec<String> anchor. No tests executed. Fixed fixture to one-element anchor vector; no production/API change.
- spec-e1-fixed-fixture.log: exit 101, 5/6 tests passed. Duplicate-anchor reader correctly refused `ambiguous anchor in <file>`; my assertion incorrectly required the anchor ID in the legacy string. Corrected assertion to the exact incumbent ambiguity diagnostic/path; no production change. This run mistakenly used repository cwd rather than required /tmp; final deciding rerun uses /tmp.
- spec-e1-final.log: exit 101, repeated 5/6. My first replacement did not match rustfmt multiline assertion, so the intended fixture correction was not saved. Verified literal match and applied exact assertion before the next targeted rerun.
