# Package scaffold defaults in the Markdown/XML tutorial

The English tutorial's `vibe init package` example now omits `--format`, so
its expected `format = "normal"` line demonstrates the actual scaffold
default. English and Russian prose describe that same behavior, while the
manifest's absent-field `simple` interpretation remains a separate contract.
Both pages cite the exact package-scaffold link and format anchors in
PROP-082. The later manual `link = "static"` edit still demonstrates the
scaffold's `dynamic` default without changing the walkthrough's output.

- `cargo test -p vibe-cli --test cli_init init_package_honours_link_and_format_defaults -- --exact`: 1/1, exit 0.
- Current `vibe doc check --examples --only 'tutorials/markdown-and-xml.xml#init-package'` against the isolated 1.1.0 English docs package and current binary: exactly 1 matched, 1 ok, 0 diverged/failed/skipped, exit 0; no golden was captured or re-blessed. Receipt: `/tmp/vibevm-next-doccheck-exact.T3WZgc/check.log`.
- Both tutorial XML files parsed; each carries `INIT-PACKAGE-FORMAT` and `INIT-PACKAGE-LINK` references (4/4).
- `git diff --check`: exit 0.

The first runner filter omitted `.xml` and returned 0 matched with exit 0; it
was not accepted as proof. A broader `init-package` filter selected two
examples (2/2), then the exact page-and-ID filter above selected one.

No product behavior, generated output, installed package or public release
changed. The M-06-A parent still needs its broader editorial/build and
native-platform evidence before closure.
