# Pi standalone package: Linux x64 acceptance boundary

Reviewed candidate `org.vibevm.agents/pi@1.5.2`, installer `linux-x64-standalone`. The manifest selects the upstream Linux x64 tarball, SHA-256 `811c6f2c9e7aaee252b5404252e556fb713aa209c1f7f924226aa77d4ef70cb5`, takes the entire `pi` directory and declares `$app_root/pi` as the executable for the public `pi` command. This option has no npm prerequisite.

The private fixture remapped the pinned supplier input locally: no live upstream download during the lifecycle run. The steps report records install exit 0; installed public `pi --version` returned exactly `1.5.2` and exit 0; repeat install replaced the selected installation; uninstall cleared its default slot; the mismatched digest produced exit 1. The terminal log ends with exit 0. Evidence: `vibevm-corp-pi-20261119T182623Z/evidence/{steps.json,installed-receipt.json,native-lifecycle.log}` on the test host.

Accepted: actual standalone binary packaging lifecycle on the tested Linux x64 host, not agent inference, the previously qualified npm carrier, Windows/macOS, network acquisition or offline redistribution rights. No binary is included in this receipt. Canonical manifest integration and registry publication are separate from this test.
