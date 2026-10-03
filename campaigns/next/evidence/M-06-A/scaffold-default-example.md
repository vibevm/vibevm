# Package scaffold defaults in the Markdown/XML tutorial

The English tutorial's `vibe init package` example now omits `--format`, so
its expected `format = "normal"` line demonstrates the actual scaffold
default. English and Russian prose describe that same behavior, while the
manifest's absent-field `simple` interpretation remains a separate contract.
Both pages cite the exact package-scaffold link and format anchors in
PROP-082. The later manual `link = "static"` edit still demonstrates the
scaffold's `dynamic` default without changing the walkthrough's output.

- `cargo test -p vibe-cli --test cli_init init_package_honours_link_and_format_defaults -- --exact`: 1/1, exit 0.
- Both tutorial XML files parsed; each carries `INIT-PACKAGE-FORMAT` and `INIT-PACKAGE-LINK` references (4/4).
- `git diff --check`: exit 0.

No product behavior, generated output, installed package or public release
changed. The M-06-A parent still needs its broader editorial/build and
native-platform evidence before closure.
