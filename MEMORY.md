# Project memory: vibevm

> **Note:** Because vibevm itself uses the `spec/boot/` layout it defines, project-level collaboration memory lives in [`spec/boot/90-user.xml`](spec/boot/90-user.xml) (the user-owned boot snippet). Any AI agent in this repo reads `CLAUDE.md` → `spec/boot/*` in order, so conventions stored there are always loaded at session start.
>
> This file is kept as a pointer so tooling that looks for a top-level `MEMORY.md` still finds its way to the right place.
