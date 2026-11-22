# ADV-CONTROL-CONTRACT-7

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe app installers <pkg> --json` leaves `portable` out of an installer row that declares none, instead of printing `"portable": null`, so the listing conforms to cli-application-installer-options.

**Write.** `crates/vibe-app/src/select.rs`, `crates/vibe-cli/tests/client_contract_app.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test client_contract_app library_installer_options_conform`

**Test.** crates/vibe-cli/tests/client_contract_app.rs: new test library_installer_options_conform. With fixture(), it runs `app installers org.example/editor --registry R --json` through one(). It asserts that the document passes common::wire::conforms::<vibe_wire::generated::application_installer_options::ApplicationInstallerOptions>, and that platforms[0].architectures[0].installers[0] has no "portable" key. Before the emitter change it fails: conforms panics with "the document and its registered wire type disagree", and the differing member is platforms ("portable": null is lost on the way back).

**Read first.**
- crates/vibe-app/src/select.rs:318-405
- crates/vibe-app/src/select.rs:460-480
- crates/vibe-cli/tests/client_contract_app.rs
- crates/vibe-cli/tests/cli_app_installers.rs:150-170

**Steps.**
1. In crates/vibe-app/src/select.rs options_listing, remove the line `"portable": option.portable,` from the json! literal of each installer row.
2. Bind that json! row to `let mut row`. After it, add `if let (Some(portable), Some(object)) = (option.portable, row.as_object_mut()) { object.insert("portable".into(), Value::Bool(portable)); }` and push `row`.
3. Add the test described in `test` to crates/vibe-cli/tests/client_contract_app.rs.
4. Run the check, then `cargo test -p vibe-app select` and `cargo test -p vibe-cli --test cli_app_installers`; both must stay green. Run cargo fmt -p vibe-app -p vibe-cli.

**Decisions already made.**
- Fix the emitter, not the schema: schemas/application_installer_options.jtd.json already declares portable optional with x-default null, which means absent when unset. An option that declares portable = true or false still prints it, so cli_app_installers.rs:163 keeps passing.
- InstallerOptionDecl.portable is Option<bool> (crates/vibe-core/src/manifest/package/application.rs:328).
- With portable removed, the rest of the real installers document already round-trips (verified).

**Not in this entry.** Not in this entry: making `portable` nullable in the schema, or changing how installation, pre_install and post_install print null (those already conform).

**Depends on.** ADV-CONTROL-CONTRACT-6

Entries that write the same file run one after another, never in parallel.
