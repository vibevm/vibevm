# F13 — Init preserves the existing host dialect

M-06-G.2, architecture L3. Ordinary package init used an unconditional Mixed target despite loading an existing XML consumer. The fix reads the existing root consumer spec_format and passes it to the incumbent generator; absent setting retains Mixed compatibility. No new type, format key/epoch, CLI flag or compiler owner.

The real XML RED had a successful command/new package but replaced STATIC.xml with STATIC.md, rewrote INDEX and three launchers. Its four installed dependency files and existing package source happened to remain unchanged; no broader mutation is claimed from that RED.

Three passing cases cover XML, explicit Markdown and absent setting with a real nonempty installed dependency tree. They compare complete path-to-byte maps for existing boot/authored content, dependencies and old package sources, plus root manifest/lock/gitignore, unrelated binary data and launcher text outside managed blocks. The new requested package is separately checked as simple/dynamic; host dialect is not inferred from those package options.

Full cli_init25 and cli_init_kinds12 pass:37 distinct tests, including marker/link/root-role controls. The final default Tool control additionally checks parsed Normal format beside Dynamic link and passes1/1; it is not counted as an extra test. Scaffold default Normal is supplied by init prompts; it is distinct from PackageFormat's backward-compatible parser default Simple. Targeted CLI/test clippy, fmt, diff and conform pass (0new,67incumbent observations). Root verified all4source hashes and13raw logs. Final map/check passes:9068spec units,4904tagged items,4060edges,0blocking findings and43known warnings.

The worker's E0599 test helper call and wrong TOML Value.parse document assertion are preserved as test-construction mistakes, separately from the valid pre-fix XML RED. No assertion was weakened. Linux execution only; native Windows qualification remains explicitly open as the old task requires. This does not implement G1 normal-lane once coverage or G3 generated-payload/source-member protection.
