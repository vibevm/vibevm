# `flow:omnichannel` — a capability lives in a library, not in a surface {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-SURFACE-DISCIPLINE A vibevm `flow` package that installs one architectural rule and the
vocabulary it needs: a capability lives in a **library**, and the command
line, the TUI, the MCP server and the HTTP API are **surfaces** over it —
thin, replaceable, and none of them the base. @status:impl/done

@fact:the-cost-appears-at-the-second-surface The cost never shows up when the first surface is written; it shows
up when the second one is, because the logic is already inside the first. @status:spec/done

@fact:TWO-SURFACES-THAT-DISAGREE-CANNOT-BE-ADJUDICATED Two surfaces of one capability that answer the same question
differently cannot be adjudicated: nothing says which is wrong. The
library is that third place. @status:impl/done

@fact:package-contents-lead This package ships: @status:impl/done

- @fact:CONTENT-THE-PROTOCOL `spec/flows/omnichannel/OMNICHANNEL-PROTOCOL.xml` — the rule, the surface
  vocabulary and its three classes, what "thin" means per class, how a
  project declares its floor, and why the coverage table must be derived. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/68-flow-omnichannel.xml` — the boot snippet read at session
  start: the rule, the vocabulary, the obligations and the never-do list. @status:impl/done

## What it is not {#not}

@fact:NOT-A-CODE-STYLE-RULE It is not a code-style rule and belongs to no language discipline. A
project writing no AI-native code still has surfaces and still pays for
letting them diverge. @status:impl/done

@fact:NOT-A-CHECKLIST-OF-SURFACES-EVERY-PROJECT-OWES It is not a checklist of surfaces every project owes. The vocabulary
names them so two projects mean the same thing; **the floor is the
project's own declaration**, and an undeclared surface is a choice rather
than a debt. @status:impl/done

## The test that settles arguments {#test}

@fact:DELETE-THE-SURFACE-ON-PAPER Delete the surface on paper. If anything but presentation and
transport is lost, the split is wrong — the capability had leaked into the
surface. @status:impl/done

