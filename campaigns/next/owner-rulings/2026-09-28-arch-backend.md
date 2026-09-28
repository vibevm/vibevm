# Initial Arch backend: pacman coexistence

Owner ruling, 2026-09-28: «Сосуществование с pacman/libalpm».

For the first Arch backend, VibeVM owns declarative composition and its own
payloads. The Arch adapter uses pacman/libalpm for native package transactions,
file ownership, dependency/version semantics and repository trust. The
immediate full-replacement branch of decision D-022 is not selected. Generic
target/backend interfaces remain independent of this choice.

This resolves the owner-choice gate for backend-specific implementation. It
does not authorize a live host-root mutation or a public publication batch.
M-15-E (the Arch backend work package) must promote the ruling into its
permanent environment/backend contract before deleting the temporary campaign
record. Revisit full replacement only on a later explicit owner instruction;
that route needs its separately planned compatibility proof.
