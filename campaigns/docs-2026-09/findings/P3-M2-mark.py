#!/usr/bin/env python3
"""A3.13 / packet P3-M2 — mark documentation obligations in ten host specs.

The only edit this script makes is to insert

    ` action="continue" actionstage="doc" audience="<audiences>"`

into the opening tag of a named fact, immediately after its `status="..."`
attribute. Nothing else in the file changes: no text, no newlines, no
indentation, no fact order, no anchor names, no new facts.

It refuses (non-zero exit, no write) when an anchor is missing, appears more
than once as an opening fact tag, or already carries an `action` attribute.
Every touched file is re-parsed with xml.etree afterwards; any parse error is
fatal.
"""

import re
import sys
import xml.etree.ElementTree as ET

# (file, anchor, audiences, why <= 12 words)
MARKS = [
    # ---- PROP-054 — the lifecycle and the extension machine ----------------
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "INSTALL-IS-CONSENT", "user",
     "Installing a package runs its code; there is no prompt"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "PHASE-BUILD", "user",
     "What the shipped `vibe build` phase actually does"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "PHASE-PACKAGE", "user",
     "What the shipped `vibe package` phase assembles"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "PHASE-DEPLOY", "user",
     "What the shipped `vibe deploy` phase applies, and where"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "LLM-IS-AN-ENHANCEMENT", "user",
     "vibe stays usable with no provider, key or agent"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "HOST-ACTIVATION", "author",
     "Manifest grammar that activates, reconfigures or disables a contribution"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "C-ABI-LAW", "author",
     "The four symbols a native extension must export"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "ABI-CRATE", "author",
     "The crate to write a native extension against"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "COMPILER-INTERNALS-FLAG", "author",
     "Without the flag a declared pass is a validation error"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "COMPILE-NATIVE-ONLY", "author",
     "Script, binary and agent handlers are refused at compile points"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "PREBUILT-CLOSED", "author",
     "How a closed-source extension ships, and when absence refuses"),
    ("vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml",
     "R8-PLATFORM-APPLICABILITY", "author",
     "The closed `when.os` vocabulary a target may declare"),

    # ---- PROP-056 — scraped project export ---------------------------------
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "SCRAPED-TREE", "user",
     "What the operation actually delivers"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "PRODUCT-PRESERVATION", "user",
     "The preservation guarantee the operator is buying"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "ZERO-RESIDUAL-SCOPE", "user",
     "What zero residue does and does not mean"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "ALGORITHMIC-BOUNDARY", "user",
     "The operator supplies policy; undecidable state refuses"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "NON-GOALS", "user",
     "Limits that stop an operator expecting more"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "SCRAPE-EXECUTION-PLATFORM-E1", "user",
     "Mutating execution is enabled only on Windows today"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "SCRAPE-PLAN-COMMAND", "user",
     "The read-only command, and what it plans by default"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "SCRAPE-EXPORT-COMMAND", "user",
     "The export command and its absent-destination rule"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "SCRAPE-INPLACE-COMMAND", "user",
     "The in-place command and its unattended requirement"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "SCRAPE-RECOVER-COMMAND", "user",
     "How a pending transaction is settled after a crash"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "SCRAPE-CONTRACT-COMMANDS", "user",
     "How a contract is created and checked"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "DEFAULT-CONTRACT", "user",
     "Where the contract lives and how its paths resolve"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "CONTRACT-STRICTNESS", "user",
     "Refusals an operator hits writing the contract"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "TYPED-ONLY", "user",
     "Globs select; only typed adapters rewrite bytes"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "AUTHORED-SPECS", "user",
     "Authored specs survive unless the contract names them"),
    ("vibevm/vibespecs/common/PROP-056-scraped-project-export.xml",
     "PRODUCT-CODE-SURVIVES", "user",
     "Accepted product code is never removed by genre"),

    # ---- PROP-024 — code-bearing packages ----------------------------------
    ("vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml",
     "PKG-PROJECT-LAW", "author",
     "A package has a project's on-disk shape"),
    ("vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml",
     "SPEC-SUBTREE", "author",
     "Where prompt content goes and how boot sources resolve"),
    ("vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml",
     "ROOT-CODE", "author",
     "Where code goes, and that it is optional"),
    ("vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml",
     "SHIPPABLE-TREE-DEF", "author",
     "Exactly what of the package directory ships"),
    ("vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml",
     "SURF-VIBEIGNORE", "author",
     "The optional file that extends the denylist"),
    ("vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml",
     "OWN-WORKSPACE", "author",
     "A code-bearing package carries its own workspace manifest"),
    ("vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml",
     "PATH-DEP-LAW", "user",
     "How a consumer references a crate shipped by a package"),
    ("vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml",
     "WORKSPACE-EXCLUDE", "user",
     "Without the exclude the consumer's own build breaks"),

    # ---- PROP-025 — binary delivery ----------------------------------------
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml",
     "BINARY-TABLE", "author",
     "The manifest table that declares a shipped tool"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml",
     "NAME-CONSTRAINTS", "author",
     "Constraints the declared name must satisfy"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml",
     "CRATE-CONSTRAINT", "author",
     "What the crate field must point at"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml",
     "BUILD-TIMING", "user",
     "Install records declarations; it does not build tools"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml",
     "BUILD-CONSENT", "user",
     "Building runs third-party build scripts without a prompt"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml",
     "BIN-BUILD", "user",
     "The command that builds a package's declared tools"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml",
     "BIN-EXEC", "user",
     "The dispatch model: the project's pinned version runs"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml",
     "OFFLINE-HONESTY", "user",
     "A first build needs the network; no offline shortcut"),

    # ---- PROP-020 — install hooks ------------------------------------------
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml",
     "INSTALLATION-CONSENT-SUCCESSOR", "user",
     "Installing a package is the consent to run its hooks"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml",
     "HOOKS-TABLE", "author",
     "The manifest table that declares a hook"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml",
     "BASE-PATH-VALUE", "author",
     "The declared value is an extensionless base path"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml",
     "SCRIPT-FORMS", "author",
     "Which script files a package must ship"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml",
     "PHASE-PRE-INSTALL", "author",
     "When the pre-install hook runs, and before what"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml",
     "PHASE-POST-INSTALL", "author",
     "When the post-install hook runs, and after what"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml",
     "CWD-IS-SLOT", "author",
     "The working directory a hook script sees"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml",
     "HOOK-ENV", "author",
     "The environment variables a hook may rely on"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml",
     "EFFECTS-EPHEMERAL", "author",
     "Hooks rerun on a nonempty diff, so must be idempotent"),

    # ---- PROP-022 — materialization modes ----------------------------------
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-022-materialization-modes.xml",
     "MODE-FIELD", "author",
     "The manifest field that selects placement"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-022-materialization-modes.xml",
     "SNAPSHOT-DEFAULT", "author",
     "The default mode, and the refused legacy spelling"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-022-materialization-modes.xml",
     "IP-REQUIRES-GIT", "author",
     "in-place is unavailable without a git source"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-022-materialization-modes.xml",
     "VENDORED-COPY-MODES", "user",
     "Which slots are committed into the project's git"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-022-materialization-modes.xml",
     "IN-PLACE-NOT-VENDORED", "user",
     "in-place slots are gitignored and need network to restore"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-022-materialization-modes.xml",
     "DESTRUCTIVE-CONFIRM", "user",
     "Destructive ops on such a slot confirm or abort"),

    # ---- PROP-011 — incremental install ------------------------------------
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml",
     "LOCKFILE-RESPECTING", "user",
     "Install honours the lock; versions never drift silently"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml",
     "UPDATE-MOVES-LOCK", "user",
     "Update is the explicit command that picks newer versions"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml",
     "BYPASS-REINSTALL", "user",
     "How to force re-materialisation of present slots"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml",
     "AUTOMATIC-NO-FLAG", "author",
     "Editing in-repo package source needs no flag"),

    # ---- PROP-012 — the managed redirect block -----------------------------
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml",
     "ONE-BLOCK-LAW", "user",
     "vibe owns exactly one block of each instruction file"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml",
     "OUTSIDE-PRESERVED", "user",
     "Everything outside the block is preserved verbatim"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml",
     "CLASS-MALFORMED", "user",
     "A malformed block aborts the run until repaired by hand"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml",
     "POSITION-USERS", "user",
     "The block's position is the user's and is never moved"),

    # ---- PROP-019 — the version manager ------------------------------------
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "VVM-IS-TOOL", "user",
     "vibe self manages the tool, not a project's packages"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "COLD-START-PATH", "user",
     "How vibe is installed on a machine that has none"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "BOOTSTRAP-SCRIPT-LATER", "user",
     "The stable install endpoints an operator actually types"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "CMD-INSTALL", "user",
     "The command that installs a selected version"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "CMD-UPDATE", "user",
     "What update refreshes, and from which origin"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "CMD-USE", "user",
     "How the active version is switched without a reload"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "CMD-ROLLBACK", "user",
     "The recovery path back to the previous version"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "ROOT-DEFAULT", "user",
     "Where installations land, and the variable that moves them"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "PATH-SHIM-DIR", "user",
     "What is added to PATH, once"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "ACTIVATION-LAW", "user",
     "Switching reloads no console and overwrites no running file"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "LAYER-ENV-ADVISORY", "user",
     "The environment variables are advisory and may lag"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "TOOLS-LIST", "user",
     "What must be installed before a source build"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "RM-PROTECTED", "user",
     "Active and running instances resist removal"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "FULLY-SCRIPTABLE", "user",
     "Every prompt has a non-interactive equivalent"),
    ("vibevm/vibespecs/common/PROP-019-version-manager.xml",
     "VARS-CONTENT", "user",
     "The command that reports the real runtime context"),
]

ATTRS = b' action="continue" actionstage="doc" audience="%s"'


def main():
    by_file = {}
    for path, anchor, audience, _why in MARKS:
        by_file.setdefault(path, []).append((anchor, audience))

    errors = []
    edited = {}

    # Bytes in, bytes out: line endings and every other byte of the file are
    # carried through untouched.
    for path, items in by_file.items():
        try:
            out = open(path, 'rb').read()
        except OSError as exc:
            errors.append('%s: cannot read (%s)' % (path, exc))
            continue
        for anchor, audience in items:
            pat = re.compile(
                b'<' + re.escape(anchor.encode()) +
                b'(\\s+fact="true"\\s+status="[^"]+")([^>]*)>')
            found = list(pat.finditer(out))
            if len(found) != 1:
                errors.append('%s: anchor %s matched %d opening fact tags (expected 1)'
                              % (path, anchor, len(found)))
                continue
            m = found[0]
            if b'action' in m.group(2):
                errors.append('%s: anchor %s already carries an action attribute'
                              % (path, anchor))
                continue
            cut = m.end(1)
            out = out[:cut] + (ATTRS % audience.encode()) + out[cut:]
        edited[path] = out

    if errors:
        for e in errors:
            print('REFUSED: ' + e, file=sys.stderr)
        sys.exit(1)

    for path, out in edited.items():
        open(path, 'wb').write(out)

    bad = []
    for path in edited:
        try:
            ET.parse(path)
        except ET.ParseError as exc:
            bad.append('%s: %s' % (path, exc))
    if bad:
        for b in bad:
            print('XML PARSE FAILED: ' + b, file=sys.stderr)
        sys.exit(2)

    per_audience = {}
    for _path, _anchor, audience, _why in MARKS:
        for a in audience.split(','):
            per_audience[a] = per_audience.get(a, 0) + 1
    print('marked %d facts in %d files' % (len(MARKS), len(edited)))
    for a in sorted(per_audience):
        print('  %-8s %d' % (a, per_audience[a]))


if __name__ == '__main__':
    main()
