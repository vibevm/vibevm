# vibevm-direct-push-smoke — PROTOCOL

Test fixture used by the manual smoke for `vibe registry publish --repo-url`.
Contents are deliberately trivial; the point is exercising the direct-push
path end to end against a real git host without burning a real package
name in any registry organisation.

## What this package would do (if it were real)

Nothing. It is a no-op flow whose only artefact is this protocol file
plus the boot snippet under `spec/boot/`. Operators who accidentally
install it should `vibe uninstall flow:vibevm-direct-push-smoke` and
forget about it.

## Versioning

Stays at `1.0.0` as a fixed fixture coordinate. A later smoke should replace
the throwaway repository rather than model product evolution here.
