# ADVANCED-ARCHITECTURE: campaign contract

`VIBEVM-ROADMAP` has three stages in order: `NST` (accepted),
`ADVANCED-ARCHITECTURE`, `VIBEVM-IDE-MVP`. This campaign delivers the useful
rest of the original architecture, restated for the current code. It was
renamed from ARCHITECTURE-REMAINDER on 2026-11-22. The previous layer-by-layer
contract is in this file's Git history.

- **Product direction (owner, 2026-11-22).** Every decision here is judged by
  what vibevm becomes: an operating system for AI and for AI developers,
  vibecoders included. Three parts, in order:
  1. *External control.* A Steam-like application (VibeVM Library, built in
     its own repository) installs and manages packages. Everything vibe does
     must be drivable by such an external application: machine-readable
     reports, progress, cancellation, unattended runs, feature detection.
  2. *Images.* vibe builds Docker images and Linux root filesystems.
  3. *The OS.* An Arch Linux based system, in some ways like SteamOS, with
     vibe as its package manager and the Steam-like application as its
     control surface. It is a cluster OS for AI: from one node it installs
     packages on other nodes. After this part comes a major release (about
     2.0.0).

  The two IDEs (the terminal one, VIBEVM-IDE-MVP, and the one inside VibeVM
  Library) exist for one purpose: a person develops for this OS with AI,
  quickly and comfortably. So the machine surfaces both of them use (reading
  specs and the tree, editor-buffer analysis, operations with progress and
  cancellation) are shared owners in vibe, built once for both IDEs and the
  Library. No IDE gets a private copy.

  The OS is an additional platform, never a replacement. Everything keeps
  working on Windows, macOS and ordinary Linux (for example Ubuntu): there
  people use the Library app with its IDE and the terminal IDE as usual. The
  portable application catalog (`vibe app`, vibe-apps) is the common base
  that works everywhere. Consequences for every capability: the core
  behaviour is portable; OS-only behaviour (pacman integration, A/B system
  updates) is an optional layer that the portable core does not depend on;
  image building works from all three desktop systems through an executor
  (for example Docker Desktop) and does not require the VibeVM OS.

  Mesh (future, owner 2026-11-22). A mesh network is being built as a
  separate component and will later move into VibeVM. Then a person adds many
  of their own computers and other compute resources to VibeVM and works with
  them as one operating system, running applications on other machines.
  Design consequence now: a target of an operation is never implicitly "this
  machine". Target identity, executors and operation records name the machine
  they act on, and the transport to another machine is pluggable (SSH first,
  the mesh later). Nothing built now may assume a single host.

  Distributed keys and inventory (future, owner 2026-11-22). Later there will
  be a distributed key store and a distributed inventory of resources, so a
  person can connect to all of them. Design consequence now: the list of named
  targets is a local view that a distributed inventory can later back, not the
  only source; credentials for a target are referenced by a handle through the
  existing secrets owner, never stored inline in target records or plans.
- **Two stages (owner, 2026-11-22).** This campaign is the base: VibeVM as a
  good system for spec-driven development, drivable by any client, on every
  desktop OS. Targets, images, cluster and the OS are a separate campaign,
  VIBEVM-PLATFORM, built on top of the base afterwards. The base builds none
  of them but keeps the invariants listed in `PLAN.md`.
- **What is planned.** The capabilities in `PLAN.md`. They were derived from
  the 222 historical task IDs (`research/inventory.json`,
  `research/capabilities.json`). Each ID has exactly one disposition there.
  Historical IDs are references for the planner. They are never entries, and
  executors never read them.
- **Outdated designs are rewritten, not dropped.** When an original task's
  form does not fit today's architecture, its user intent is restated in
  today's commands and crates. An item is dropped only when no user-visible
  behaviour remains, or when today's product already serves it.
- **No parallel engines.** Every capability extends an existing owner. There
  is no new resolver, verdict store, journal, service or proof layer.
- **Milestones are goals with closed lists.** Each milestone in `PLAN.md` is
  one plan node whose children are entries prepared per
  `working-rules/rules/decomposing.md`. A bounded lead may execute them.
- **Owner gates.** ADV-RELEASE (CI edits), ADV-CONFIG (Redbook publication)
  and ADV-TARGETS (product direction B-233) wait for the owner's word.
  Publishing a release is never part of an entry.
- **Done.** The campaign is accepted when each milestone's list is closed
  (accepted, dropped or deferred, each with a reason). Then the owner map goes
  to VIBEVM-IDE-MVP.
