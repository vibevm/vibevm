# Quiet contribution output — accepted logging contract alignment

2026-11-19; architectural decision relayed by vibevm-architect. Separate from Hermes/Pi native blockers and preset inventory repair16cf7329421c4f679845c2364ca5d130c4359a99.

The jobs2 baseline reached cli_hook_rerun and failed its older direct-quiet inherited-output expectation (4 PASS/1 FAIL). Investigation found an actual contract conflict: historical5506cf881 test and observer prose promised raw inherited output, while accepted full-log implementationddc7e6863 and cli_install_slot_steps explicitly suppress raw child output under quiet and retain full raw logs. The newer test independently passed. No raw-log/stream product behavior was changed to satisfy one side.

Architect selected newer accepted logging behavior as canonical: direct install --quiet suppresses raw child stdout/stderr at the terminal, retains every raw byte in contribution logs and prints its summary; JSON stays machine-clean and composed quiet remains suppressed. The narrow PROP-065 display sentence and stale observer prose are aligned accordingly. The old direct quiet test is renamed and asserts successful execution, exactly one summary line, no marker on stdout/stderr, retained full raw marker and hook execution count exactly one. No ignored test, weakened equality, disabled logging or native scope waiver.

Essential gates: entire cli_hook_rerun target (includes composed quiet/JSON), cli_install_slot_steps full-log/quiet regressions, format/diff and independent architect spec/diff review. Exact terminal results will be bound in the candidate manifest. Prior failed logs remain historical RED; no full integration PASS is inferred from these focused repairs.
