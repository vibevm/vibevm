//! DRIFT-016 — losing the payload sidecar is harmless *and* silent.
//!
//! The in-process tests in `src/commands/progress/tests.rs` pin the store
//! with `[progress] cache_dir`. This one deliberately does not: it
//! exercises the **default** location under the per-user settings home,
//! which is the path every real run takes, and it does so out of process
//! so that "emits nothing about it" can be asserted on actual stderr
//! rather than argued from the source.
//!
//! Hence [`UserScratch`]: the settings home is relocated to a temp tree,
//! so the store can be deleted wholesale by name and the developer's real
//! `~/.vibe` is never read or written. One variable does that — which is
//! exactly why the sidecar has none of its own (F-055: the harness that
//! isolated `VIBE_REGISTRY_CACHE` and forgot the settings chokepoint).

mod common;

use std::path::Path;

use common::UserScratch;

/// Two observed files with enough shape that a lossy reuse would show,
/// plus an empty campaign zone. No `[progress] cache_dir`: the point here
/// is the default.
fn fixture(root: &Path) {
    std::fs::create_dir_all(root.join("spec")).expect("mkdir spec");
    std::fs::write(
        root.join("spec/a.md"),
        "<status stage=\"impl\" state=\"work\"/>\n\n\
         # Alpha {#alpha}\n\n\
         ##a1 The first claim. @test/plan\n\n\
         - ##a2 An item. @doc/done\n\
         - ##a3 Another item. @impl/hold\n",
    )
    .expect("write a");
    std::fs::write(
        root.join("spec/b.md"),
        "# Beta {#beta}\n\n\
         <status stage=\"spec\" state=\"plan\"/>\n\n\
         ##b1 A paragraph under the section marker. @spec/work\n",
    )
    .expect("write b");
    std::fs::write(root.join("progress.toml"), "include = [\"spec/**/*.md\"]\n")
        .expect("write cfg");
    std::fs::create_dir_all(root.join("campaigns/progress-test/run")).expect("mkdir campaign");
}

/// `updated_at` is a wall clock and would make any equality below a coin
/// flip; everything else is compared byte for byte.
fn blank_stamps(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    for (i, part) in json.split("\"updated_at\": \"").enumerate() {
        if i > 0 {
            out.push_str("\"updated_at\": \"<stamp>");
            match part.find('"') {
                Some(end) => out.push_str(&part[end..]),
                None => out.push_str(part),
            }
        } else {
            out.push_str(part);
        }
    }
    out
}

fn state(root: &Path, name: &str) -> String {
    blank_stamps(
        &std::fs::read_to_string(root.join("campaigns/progress-test/run/state").join(name))
            .unwrap_or_else(|e| panic!("read {name}: {e}")),
    )
}

/// The erasure law with teeth (§4.3): a scan whose payload store was
/// deleted answers byte for byte what the warm one answered, and says
/// nothing at all about the store it did not find.
#[test]
fn sidecar_absent_runs_cold() {
    let scratch = UserScratch::new();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    fixture(root);

    let scan = || {
        scratch
            .vibe()
            .args(["progress", "scan", "--path"])
            .arg(root)
            .output()
            .expect("run vibe progress scan")
    };

    // Warm the store up: this run parses everything and leaves a sidecar
    // behind, in the default location.
    let first = scan();
    assert!(first.status.success(), "first scan: {first:?}");
    let store_root = scratch.settings.join("progress-cache");
    assert!(
        store_root.is_dir(),
        "the default store lands under the settings home, not beside the repo"
    );
    let warm_corpus = state(root, "corpus.json");
    let warm_campaign = state(root, "campaign.json");

    // …and now it is gone, the way a fresh clone or a cleaned home has it.
    std::fs::remove_dir_all(&store_root).expect("erase the store");

    let cold = scan();
    assert!(cold.status.success(), "scan without a sidecar: {cold:?}");
    assert_eq!(
        String::from_utf8_lossy(&cold.stderr),
        "",
        "a missing payload store is not a warning, an error, or a mention"
    );
    assert_eq!(state(root, "corpus.json"), warm_corpus, "corpus.json");
    assert_eq!(state(root, "campaign.json"), warm_campaign, "campaign.json");
    // The run that missed also rebuilt what it missed — the store is an
    // accelerator that repairs itself, never a thing to restore by hand.
    assert!(store_root.is_dir(), "the run wrote the store back");
}
