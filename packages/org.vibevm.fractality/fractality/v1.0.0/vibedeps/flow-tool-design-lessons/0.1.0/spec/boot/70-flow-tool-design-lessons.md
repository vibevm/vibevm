# Flow: Tool Design Lessons {#root}

This project builds a tool that manages itself — a self-updating CLI,
an installer, a version manager, or a package system. The **tool
design lessons** catalog is installed: numbered, self-contained
lessons, each one paid for by shipping such a tool and the ecosystem
around it.

## When to read {#when}

Before you design an activation model, an install pipeline, an
identity scheme, a durable-environment edit, or a package format, read
the lesson that governs it **first**. The law is one line; the
rationale is why it is not negotiable.

- Self-updating tools — activation, instances, identity, environment
  edits, removal:
  [`spec/flows/tool-design-lessons/self-updating-tools.md`](../flows/tool-design-lessons/self-updating-tools.md).
- Packaging — what ships, what identity is, the bootstrap:
  [`spec/flows/tool-design-lessons/packaging-lessons.md`](../flows/tool-design-lessons/packaging-lessons.md).

The index and the cross-cutting maxims:
[`spec/flows/tool-design-lessons/TOOL-DESIGN-LESSONS.md`](../flows/tool-design-lessons/TOOL-DESIGN-LESSONS.md).

## Never {#never}

- Never make an environment variable the source of truth for the
  active version — env is frozen at process start; read a live pointer
  file each launch instead.
- Never overwrite a file that may be in use — write a new instance
  directory and flip a pointer.
- Never content-hash gigabytes to establish identity — count instances
  and detect change cheaply.
- Never ship prose describing tooling the consumer does not receive —
  ship the runtime.
- Never let a package's identity include build artifacts — identity is
  the source.
