# tool: jtd-codegen — the vendored wire-type generator {#root}

<status stage="impl" state="done"/>

@fact:WHAT-IT-IS `jtd-codegen` generates strictly-typed, language-specific code
from JTD ([JSON Type Definition, RFC 8927](https://www.rfc-editor.org/rfc/rfc8927))
schemas. A consuming project derives its wire types from committed
`*.jtd.json` schemas through this binary — schema-first codegen instead of
hand-maintained duplicates on either side of a contract. @status:impl/done

@fact:THIS-PACKAGE-IS-THE-RECIPE-NOT-THE-BINARY **This package ships the
provisioning recipe, never the binary.** The binary is fetched from the
upstream release page into the consuming project's local
`tools/jtd-codegen/` directory, which the consumer keeps gitignored — only
the recipe (this document) and the version pin travel with source trees.
That is the same posture the binary's first consumer records in its own
`tools/.gitignore`: toolchain binaries are vendored per machine, never
committed. @status:impl/done

@fact:UPSTREAM Upstream:
<https://github.com/jsontypedef/json-typedef-codegen>. @status:impl/done

## Pinned version {#pin}

@fact:PINNED-VERSION **`jtd-codegen 0.4.1`** — the most recent stable release at
the time of pinning. This README is the pin's single home: bump by editing
this line (a new package version), never by restating the number in a
consumer tree. CI asserts that schemas do not drift from generated code;
it does not enforce a particular generator build. @status:impl/done

## Install {#install}

@fact:INSTALL-TARGET Drop the platform binary at
`tools/jtd-codegen/jtd-codegen` (or `jtd-codegen.exe` on Windows) inside
the consuming project, and keep that directory gitignored. @status:impl/done

### Windows {#install-windows}

@fact:INSTALL-WINDOWS From the project root, in PowerShell or Git Bash: @status:impl/done

```sh
curl -LO https://github.com/jsontypedef/json-typedef-codegen/releases/download/v0.4.1/x86_64-pc-windows-gnu.zip
unzip -d tools/jtd-codegen x86_64-pc-windows-gnu.zip
rm x86_64-pc-windows-gnu.zip
```

@fact:WINDOWS-GNU-BUILD Upstream ships a `gnu` build, not `msvc` — the static
binary works on the Windows hosts the projects target. @status:impl/done

### macOS {#install-macos}

@fact:INSTALL-MACOS Apple Silicon (Intel: swap in
`x86_64-apple-darwin.tar.gz`): @status:impl/done

```sh
curl -L https://github.com/jsontypedef/json-typedef-codegen/releases/download/v0.4.1/aarch64-apple-darwin.tar.gz \
  | tar -xz -C tools/jtd-codegen
```

### Linux {#install-linux}

@fact:INSTALL-LINUX One command: @status:impl/done

```sh
curl -L https://github.com/jsontypedef/json-typedef-codegen/releases/download/v0.4.1/x86_64-unknown-linux-gnu.tar.gz \
  | tar -xz -C tools/jtd-codegen
```

### Verify {#verify}

@fact:VERIFY-VERSION The installed binary answers with the pinned version: @status:impl/done

```sh
tools/jtd-codegen/jtd-codegen --version    # prints "jtd-codegen 0.4.1"
```

## Use {#use}

@fact:CONSUMER-WIRES-ITS-OWN-TASK The consumer wires the binary into its own
regeneration task and drift check — generate, then byte-compare in CI. The
first consumer (the vibevm host) runs it as `cargo xtask codegen` /
`cargo xtask check-codegen`, preferring the project-local binary and
falling back to PATH; its task preflights the binary and errors
actionably, pointing at this recipe, when both are missing. @status:impl/done
