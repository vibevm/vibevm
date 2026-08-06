# Flow: Tool Design Lessons {#root}

<status stage="impl" state="done"/>

@fact:PROJECT-BUILDS-A-TOOL-THAT-MANAGES-ITSELF This project builds a tool that manages itself — a self-updating CLI,
an installer, a version manager, or a package system. @status:impl/done

@fact:THE-LESSONS-CATALOG-IS-INSTALLED The **tool
design lessons** catalog is installed: numbered, self-contained
lessons, each one paid for by shipping such a tool and the ecosystem
around it. @status:impl/done

## When to read {#when}

@fact:READ-THE-GOVERNING-LESSON-FIRST Before you design an activation model, an install pipeline, an
identity scheme, a durable-environment edit, or a package format, read
the lesson that governs it **first**. @status:impl/done

@fact:the-law-is-one-line-the-rationale-is-why The law is one line; the
rationale is why it is not negotiable. @status:impl/done

- @fact:POINTER-SELF-UPDATING-TOOLS Self-updating tools — activation, instances, identity, environment
  edits, removal:
  @spec://org.vibevm.world/tool-design-lessons/flows/tool-design-lessons/self-updating-tools#root. @status:impl/done
- @fact:POINTER-PACKAGING-LESSONS Packaging — what ships, what identity is, the bootstrap:
  @spec://org.vibevm.world/tool-design-lessons/flows/tool-design-lessons/packaging-lessons#root. @status:impl/done

@fact:index-and-maxims-pointer The index and the cross-cutting maxims:
@spec://org.vibevm.world/tool-design-lessons/flows/tool-design-lessons/TOOL-DESIGN-LESSONS#root. @status:impl/done

## Never {#never}

- @fact:NEVER-MAKE-AN-ENVIRONMENT-VARIABLE-THE-SOURCE-OF-TRUTH Never make an environment variable the source of truth for the
  active version — env is frozen at process start; read a live pointer
  file each launch instead. @status:impl/done
- @fact:NEVER-OVERWRITE-A-FILE-THAT-MAY-BE-IN-USE Never overwrite a file that may be in use — write a new instance
  directory and flip a pointer. @status:impl/done
- @fact:NEVER-CONTENT-HASH-GIGABYTES-TO-ESTABLISH-IDENTITY Never content-hash gigabytes to establish identity — count instances
  and detect change cheaply. @status:impl/done
- @fact:NEVER-SHIP-PROSE-DESCRIBING-TOOLING-THE-CONSUMER-DOES-NOT-RECEIVE Never ship prose describing tooling the consumer does not receive —
  ship the runtime. @status:impl/done
- @fact:NEVER-LET-A-PACKAGES-IDENTITY-INCLUDE-BUILD-ARTIFACTS Never let a package's identity include build artifacts — identity is
  the source. @status:impl/done
