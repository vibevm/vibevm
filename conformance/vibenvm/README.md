# vibenvm differential conformance suite

That vibe is a complete nvm replacement is proven **algorithmically, never by
an LLM's judgement or review** (PROP-075 `#VIBENVM-ALGORITHMIC-PROOF`). This
directory is that proof: the same command sequences, request for request,
against a pinned upstream nvm-sh and against vibe's `vibenvm` shim, each in its
own isolated home and shell, from the same starting state and the same local
mirror of Node release files, compared observation record by observation record
after every step.

**The verdict is the exit code.** Nothing here reads or writes a model's
opinion, and output text is compared only through declared, mechanical
normalisations — and only when a case asks for it.

Linux only, and that is a statement of scope, not an oversight: the network
isolation is `bwrap --unshare-net` and the proof that nothing reached past
loopback is `strace -e trace=connect`. It is not a cross-platform claim.

## Running it

```sh
cd <repo>
conformance/vibenvm/fetch-mirror.sh          # once; the ONLY networked step
cargo build -p vibe-cli                      # sandbox B's tool

# The two packages sandbox B installs live in their own repositories, so
# they are published into a local `localdir` registry first. The defaults are
# /fast/git/vibe-pkg-node and /fast/git/vibe-pkg-vibenvm; pass your own.
conformance/vibenvm/make-registry.sh
export VIBENVM_REGISTRY="$PWD/conformance/vibenvm/mirror/registry"
export VIBE_BIN="$PWD/target/debug/vibe"     # or wherever you built it

cargo xtask vibenvm-conform ; echo "verdict=$?"
conformance/vibenvm/run.sh --self-test ; echo "selftest=$?"
```

`run.sh` takes the same two as flags — `--vibenvm-registry PATH` and
`--vibe PATH` — and the environment variables exist because
`cargo xtask vibenvm-conform` passes only `--cases` through.

`fetch-mirror.sh` reaches exactly two hosts, `nodejs.org` and `github.com`,
sends no credentials, and writes only into the git-ignored `mirror/`. Every
file it downloads is pinned by SHA-256 in the committed `mirror.lock`, and the
suite refuses to run on a mismatch.

| exit | meaning |
|---|---|
| 0 | every case passed and coverage is complete |
| 1 | a differential mismatch — the case, the step, the field and both values are printed |
| 2 | coverage is incomplete, or the exclusion list is not what the owner approved |
| 3 | a harness error: a mirror digest mismatch, a missing pinned tag, a missing prerequisite, a **non-loopback connection attempt**, or a sandbox that could not be built |

Useful flags: `--cases GLOB` (one case or a family), `--list`, `--keep` (keep
the work directory and say where it is), `--work DIR`, `--self-compare`,
`--vibe PATH`, `--vibenvm-registry PATH`. Prerequisites: `bwrap`, `strace`,
`python3` (3.11+, for `tomllib`), `curl`, `git`, `tar`, `timeout`,
`sha256sum`.

Budget: the suite installs real Node versions from the local mirror, twice per
case (once per sandbox), so a full run is minutes, not seconds. Cases carry
their own `wall_secs`; `16-default-packages` and `14-reinstall-packages-from`
are the slowest, because they are the ones that run a real `npm install -g`.

## What each sandbox is

Sandbox A is the pinned nvm-sh checkout as its own `$NVM_DIR`, pointed at the
loopback mirror with `NVM_NODEJS_ORG_MIRROR`.

Sandbox B is a vibe home of its own: its own `HOME` and `VIBE_SETTINGS`, a
`[[remap]]` row sending `https://nodejs.org/dist` to the same loopback mirror
(PROP-066 `#locator` — the mechanism a closed network uses anyway, so the
suite exercises the real path rather than a test-only flag), a `[[registry]]`
row declaring the local `localdir` registry as that home's own, and
`vibe install -g org.vibevm.node/vibenvm` from it. The shim is then sourced
from the **installed** copy under `<settings>/opt/`.

The registry is configured in the home rather than passed as `--registry` on
one command, because the shim calls `vibe install -g org.vibevm.node/node@…`
exactly as a user's shell would and must not carry a test-only flag. `vibe`
itself is on sandbox B's `PATH` — that is how the shim finds it and how a
user has it — in a directory **outside** the tool root, so it never appears
in the `owned_path` observation. Sandbox A's `PATH` carries nvm the way nvm
is carried: as a sourced function.

A sandbox that cannot be built is a **harness error (exit 3)** naming what is
missing — never a pass and never a mismatch: a suite that could report
conformance it never measured would be worth nothing.

## How it works

**Isolation.** One temporary root per case, two sandboxes inside it. Both start
from a `PATH` reduced to the fixed list `/usr/bin:/bin`, with every `NVM_*` and
`VIBE_*` variable of the invoking environment removed (`env -i`), their own
`HOME` and `TMPDIR`, and `LC_ALL=C`. Sandbox A holds the pinned nvm-sh checkout
as its `$NVM_DIR`; sandbox B holds its own `VIBE_SETTINGS`.

**No network at all.** `run.sh` re-executes itself inside
`bwrap --unshare-net`, where only loopback exists, and serves `mirror/` over
loopback HTTP. Sandbox A is pointed at it with `NVM_NODEJS_ORG_MIRROR` and
sandbox B with its remap row. Every case runs under
`strace -f -e trace=connect`, and **any** `connect(2)` that is not a unix
socket, netlink, `127.0.0.0/8` or `::1` fails the run with exit 3 — even
though, inside that namespace, it could not have succeeded. A promise is not a
measurement. npm's registry and nvm's io.js mirror are also pointed at
loopback directories, so a command that wants either gets a deterministic 404
instead of a forbidden connection.

**One shell per case.** A case's steps run in a single shell, in order, because
the state a version manager exists to change — `PATH` above all — has to carry
from step to step the way it does for a user.

**The observation record** (`observe.sh`), written after every step in each
sandbox, is the owner's own list mechanised: `exit`, `node_v`, `npm_v`,
`which_node`, `owned_path` (the ordered `PATH` entries under the tool's own
root), `installed`, `default`, `aliases`, and `globals` (for each installed
version, its global npm packages as sorted `name@version` pairs). The record
is read from disk and from vibe's own query commands — never by asking the tool
under test to grade itself. **The comparison is of these records, field by
field**; stdout and stderr are not compared unless a case sets
`compare_stdout = true`, which is what keeps an LLM out of the verdict.

Three of those fields are read as *resolved* answers rather than as each
tool's storage, and each has a reason:

- **`default`** is the version a new shell would actually get. nvm's `default`
  is an alias file that may name another alias (`default` → `lts/*` →
  `lts/jod`) and that survives the uninstall of the version it names; the
  record follows the chain exactly as nvm's own `nvm_resolve_alias` does, and
  reports `absent` when what it lands on is not installed, because a default
  naming a version that is gone provides nothing.
- **`aliases`** is every name the tool offers with what it resolves to. nvm
  keeps an alias file per LTS line; vibe keeps no alias store at all and
  computes the same names from the version list. `lts` is vibe's name for
  "the newest LTS of any line", which is nvm's `lts/*` — one name under two
  spellings. The channels are asked for **offline**, so the observation makes
  no request of its own and reports only the names the tool has already
  fetched, which is exactly what nvm's alias store is: before any remote call
  both tools answer `none`.
- **`installed`** and sandbox B's `default` come from `vibe app list -g
  --json` (`vibe-app-list/1`), the home's own inventory, which needs no
  registry, no template and no request — the counterpart of reading nvm's
  `versions/node` directory on disk.

**Normalisations** are declared in `normalise.rules` and nowhere else: sandbox
paths to `<HOME>`, the tool root to `<TOOLROOT>`, ANSI colour stripped, times
to `<T>`, progress and npm-notice lines dropped, and the set-valued fields
sorted. Two rules earn their own sentences. A version's directory under the
tool root collapses to `<TOOLROOT>@v<version>`, so `which_node` and
`owned_path` compare as what they are — the directory of a specific version —
without either tool's layout winning, whether the version is a path component
of its own (`versions/node/v22.9.0`) or the version half of a coordinate
(`apps/node/org.vibevm.node.node@22.9.0`). And a leading `v` is dropped from a
version that starts a field or a member of one, because nvm writes `v22.9.0`
where vibe writes `22.9.0` and nvm's own `nvm_version` adds and strips it
freely: the `v` is a spelling, not an observable.

**The coverage gate.** `extract-inventory.sh` re-extracts nvm's inventory from
the pinned tree — the dispatch `case` patterns of the `nvm()` function, the
flag `case` patterns inside each branch, and the text `nvm --help` prints —
into `inventory.json`. `coverage.py` then requires every `(subcommand, flag)`
pair in it to be covered by at least one **passing** case or to appear in
`exclusions.toml` as an owner-approved row. Coverage is derived from the
command lines the cases actually ran, not declared by hand: a case covers what
it runs, and only if it passed.

The inventory is extracted rather than authored for one reason — a later nvm
release cannot pass unnoticed. Re-extract with
`cargo xtask vibenvm-conform --update-inventory` or `./extract-inventory.sh`,
and the diff of `inventory.json` is the new surface.

Today: **76 pairs** from nvm v0.41.0 — 52 required and covered by the cases,
24 excluded by the owner's twelve approved items. Measured 2026-11-11 against
the real shim: nine cases passed, 52 of 52, **`verdict=0`**.

**The exclusion list is closed.** `exclusions.toml` carries exactly the owner's
decision of 2026-11-10 (PROP-075 `#VIBENVM-EXCLUSIONS-APPROVED`), one reason
per row, `approved_by = "owner"` and the date. A row without that approval
fails the gate, and so does a row outside the closed set — `coverage.py` holds
that set, so a worker can neither widen the list nor quietly reword it. The
two exclusions the owner **rejected**, `install --skip-default-packages` and
`$NVM_DIR/default-packages`, are covered by a real case
(`cases/16-default-packages.toml`) instead.

## The meta-tests

`run.sh --self-test` runs sandbox A against **itself** over every case — which
proves the comparison, the normalisations and the coverage gate agree with a
tool that is conformant by construction — and then runs the fixtures that
prove the verdict can fail:

| fixture | must give |
|---|---|
| a deliberately perturbed observation field | 1, naming the case, step, field and both values |
| a pair no case covers | 2, naming the pair |
| an exclusion row without `approved_by = "owner"` | 2 |
| an exclusion row beyond the owner's closed set | 2 |
| a tampered `mirror.lock` | 3 |
| a real non-loopback `connect(2)` attempt | seen by the guard (3 in a run) |

The perturbation is the one place in the suite where anything is deliberately
wrong: `VIBENVM_PERTURB_FIELD` is honoured by `observe.sh` for the second
sandbox and is set by nothing but the meta-tests.

## Files

| file | what it is |
|---|---|
| `run.sh` | the suite; its exit code is the verdict |
| `fetch-mirror.sh` | the one networked step: the mirror and the pinned nvm-sh |
| `mirror.lock` | the pin (nvm tag object, node version set) and every mirrored file's SHA-256 |
| `extract-inventory.sh` → `inventory.json` | nvm's own command and flag inventory, extracted from the pinned tree |
| `exclusions.toml` | the owner-approved accepted-gap set, with a reason per row |
| `coverage.py` | the coverage gate, and the lock on the exclusion list |
| `observe.sh` | one observation record per step per sandbox |
| `normalise.rules` | every normalisation the comparison applies |
| `cases/*.toml` | the differential cases |
| `nvm-tests.allow` | the bounded, recorded subset of nvm's own `test/fast` that may be reused |
| `make-registry.sh` | publishes the two packages sandbox B installs into a local `localdir` registry |
| `mirror/` | git-ignored: the downloaded mirror, the pinned checkout and (by default) that registry |

### Writing a case

```toml
schema = 1
name = "install-use-uninstall"
summary = "what this case is for, and what it would catch"
wall_secs = 900
compare_stdout = false          # default: records only
observe_files = [".nvmrc"]      # these files' content joins the record
setup = ["..."]                 # harness scaffolding, both sandboxes
setup_a = ["..."]               # sandbox A's own way of declaring something
setup_b = ["..."]               # sandbox B's way of declaring the same thing
commands = ["nvm install 22.9.0", "nvm use 22.9.0"]
```

`nvm` is the tool under test: in sandbox A it is the pinned function, in
sandbox B the shim's. `setup_a` / `setup_b` exist because the same *wish* is
sometimes declared differently by the two tools — nvm's `default-packages`
file versus the user's `[slot.node.extras.npm] add` list — and comparing the
declarations would be comparing the wrong thing. The `commands` are always
identical.

## As built — what the implementation decided

Nine notes, each one a decision a reader would otherwise have to
reverse-engineer. The first four are the harness half's; notes 5–9 are what
plugging the real shim in decided.

1. **The owner's twelve approved exclusion items are fourteen rows**, because
   `#VIBENVM-EXCLUSIONS-APPROVED`'s own row rule (1) makes an item that names
   two flags two rows (`install -s` and `install -b`), and the internal-state
   item is recorded under both the `(environment)` and the `(internal)`
   spelling that fact prescribes. Each row carries `item`, so the twelve stay
   countable; `coverage.py` checks both the row set and the item count.
2. **The mirror's indexes are filtered** to the pinned version set. A mirror
   describes what the mirror has — and an unfiltered `index.json` changes with
   every Node release, so it could never be pinned by digest.
3. **The observation record gained one field family, `file:<path>`**, driven
   by a case's `observe_files`. Without it `install --save` could only be
   observed as "exited 0": the version file it wrote would not be compared.
   The owner's own note says adding to the observation list is cheap because
   the comparison is field-by-field; this is that.
4. **Flag spelling in the inventory and the exclusions carries no `=` and no
   value**: nvm's `--lts` and `--lts=*` are one flag. A branch's flags are
   recorded against the branch's first (canonical) subcommand spelling, and an
   alias spelling (`i`, `list`, `copy-packages`) gets a bare pair of its own,
   so the gate still asks for a case per alias.
5. **The three things the harness half said to confirm: one held, two did
   not.** The installed package does publish a `vibenvm.sh` to source, and
   `run.sh` now takes it from under `<settings>/opt/` rather than by name
   anywhere under the settings root — the package cache holds a copy of the
   same bytes, and sourcing *that* would let a failed installation pass. The
   `<TOOLROOT>@v<version>` collapse did **not** cover vibe's layout: it
   required the version to be a path component (`/v22.9.0`), while vibe's is
   the version half of a coordinate (`…node@22.9.0`), so the rule now accepts
   either separator. And sandbox B's readers named fields vibe does not emit —
   `vibe app env --json` carries no `default` and `app versions --json` nests
   its versions under `packages[]` — so they were rewritten against the
   documents vibe actually emits, which also moved `installed` and `default`
   onto `vibe app list -g --json`, the one answer that needs no registry and
   no request.
6. **Every version a *target* names is spelled `=<version>`.** A bare
   `node@22.9.0` is a *requirement*, so `vibe app exec node@22.9.0` runs the
   newest installed version that satisfies it — 22.11.0 when that is
   installed. An observation taken that way would silently read a different
   version than the one it asked about, and the shim would install a different
   version than the user typed. This is the single sharpest thing the
   differential proof caught.
7. **`aliases` and `default` are compared resolved, not as stored**, and
   sandbox B's channels are asked for offline. The three reasons are in *The
   observation record* above; all three are about comparing what a user can
   see rather than how each tool spells its own storage.
8. **The two whole-list `ls-remote` spellings name nvm's own `node`
   flavour.** A bare `nvm ls-remote` lists "node AND io.js" and returns the
   io.js half's status, which against this suite's empty loopback io.js
   mirror is 3 — so the step's exit status would be decided entirely by nvm's
   io.js category, an owner-approved accepted gap that vibe has no concept of
   at all. Naming the flavour keeps a required pair's observable inside the
   surface both tools have, and covers the same pair, because `coverage.py`
   reads flags only from the tokens directly after the subcommand and `node`
   is not a flag. The case says so in its own summary.
9. **Each sandbox's `PATH` carries its own tool.** Sandbox A has nvm as a
   sourced function; sandbox B needs `vibe` as a program, so `run.sh` links it
   into a directory of that sandbox — deliberately **outside** the tool root,
   so it never appears in `owned_path`. The shim then finds vibe the way a
   user's shell does, and carries no test-only variable.
