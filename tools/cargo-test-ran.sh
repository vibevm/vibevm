#!/usr/bin/env bash
# Run `cargo test` with the given arguments and succeed only when at least one
# test ran and every test passed.
#
# Why: a `cargo test <filter>` whose filter matches nothing exits 0, so a
# completion-list check written against a test that does not exist yet would
# "pass" before any work is done. Entry checks use this wrapper instead of a
# bare `cargo test`, so "fails before, passes after" holds by construction.
#
# Usage: bash tools/cargo-test-ran.sh -p <crate> [--lib|--bin X|--test Y] [filter] [-- <libtest args>]
set -u
output=$(cargo test "$@" 2>&1)
status=$?
printf '%s\n' "$output"
passed=$(printf '%s\n' "$output" | sed -n 's/^test result: [a-zA-Z]*\. \([0-9][0-9]*\) passed.*/\1/p' | awk '{s += $1} END {print s + 0}')
if [ "$status" -ne 0 ]; then
    echo "cargo-test-ran: cargo test failed (exit $status)" >&2
    exit "$status"
fi
if [ "$passed" -eq 0 ]; then
    echo "cargo-test-ran: no test ran; the filter matched nothing" >&2
    exit 1
fi
echo "cargo-test-ran: $passed test(s) passed"
