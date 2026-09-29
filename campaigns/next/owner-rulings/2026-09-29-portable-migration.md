# Portable migration concurrency contract

Owner ruling, 2026-09-29: migration must work on Linux, macOS and Windows.
The owner explicitly accepts a weaker concurrency contract on platforms where
full guarantees cannot be provided. Lack of unrestricted content-CAS alone
must not disable normal Linux use.

M-13-D.1 must declare the actual writer/barrier scope. Participating migration
clients serialize through a common barrier and compare exact approved source
bytes before publication. Advisory locks and read-then-rename checks must not
be described as unconditional content-CAS against unrelated processes.
Uncooperating writers, pre-existing writable handles and observable partial
states remain explicit limits where the implementation cannot exclude them.
Document these limits and expose truthful structured guarantee/warning and
conflict/effect information. Do not promise that every external race can be
detected, or that a returned error proves the file was untouched.

The weaker contract does not waive retained approved before-images, durable
intent, validation, bounded path authority, conflict checks, interruption
inspection, idempotent replay or truthful recovery. Unknown/third states are
preserved and require explicit resolution; automatic recovery must not claim
a successful rollback without evidence. Genuine unsupported filesystems or
failed durability operations may still refuse with their precise reason.

Provide working implementations for all three platforms; distinguish tested
Linux evidence from macOS/Windows implementation and later native proof.
M-16 may later add participating-buffer barriers without changing the meaning
of the declared guarantee. No live host/user migration, fixture/upstream
rewrite, public publication or version increment is authorized by this ruling.
Product and active own packages remain mutable 1.1.0 under the existing hold.

This ruling supersedes the unrestricted exact-CAS assumption of M-13-D.1.
Promote the product contract to PROP-044 before implementing it; retire this
campaign-only record only after permanent promotion and matching proof.
