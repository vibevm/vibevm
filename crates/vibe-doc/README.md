# vibe-doc

The documentation pipeline of VibeVM: one library that owns everything
with content in it, so the CLI, the MCP tools and the local reader are
thin projections over a single implementation rather than three of them
(`spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY`).

What it holds today:

- **`pages`** — the one reader of a documentation package. Pages live
  under `vibevm/vibespecs/` and are parsed by the `vibe-specdoc` pivot
  with the documentation vocabulary open. The pivot will not choose that
  vocabulary itself — it knows nothing of package kinds, by law — so this
  is where the choice is made
  (`spec://org.vibevm.core/vibevm/common/PROP-045#DOC-VOCAB-BY-KIND`).
- **`examples`** — the example runner behind `vibe doc check --examples`.
  Every documented command runs against the built binary in a fresh
  sandbox and its output is compared exactly, after the normalisation its
  fixture declares. No match templates, no shell, no writes outside the
  sandbox.
- **`derived`** — the generators behind `vibe doc check --derived`:
  command help, JTD field tables and manifest fields, produced at build
  time and never committed as page text.

Fixtures live with the documentation they serve, in the package's own
`examples/<fixture>/` directory: a recipe (`example.toml`) that says what
the sandbox looks like before the command runs, which differences the
comparison may ignore, and which JTD schema each `--json` document must
satisfy.

Separability: this crate depends on the pivot and on nothing else of
vibevm. It takes the binary to run, the sandbox root and the paths its
tripwire guards as data from its caller; it reads no ambient environment
of its own.
