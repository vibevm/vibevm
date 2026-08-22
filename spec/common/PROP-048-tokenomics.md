# PROP-048 — Tokenomics: the cost of text is a design law {#root}

<status stage="spec" state="done" action="continue" comment="the owner's founding declaration, 2026-08-22: token economy is CENTRAL to all of VibeVM; §3 declares STATIC as the cache-stable prefix (the build waves it names stay spec/plan); §4 inventories the mechanisms already serving the law; §5 is the open direction list"/>

## 1. Mandate {#mandate}

@fact:TOKENOMICS-MANDATE **The owner's declaration (2026-08-22, chat,
near-verbatim): «Я хочу, чтобы идея эффективной токеномики была центральной
для всего VibeVM. Мы манипулируем чудовищно огромным объёмом текста промтов —
и мы должны сделать всё, чтобы пользователь платил за них меньше.»** VibeVM's
whole subject matter is prompt text at scale: boot lanes, spec corpora,
materialised dependencies, worker packets, campaign weaves. Every one of those
is tokens the user pays for — so their cost is not an operational detail but a
first-class design axis of the system. @status:spec/done

## 2. The law {#law}

@fact:TOKENOMICS-IS-A-DESIGN-PRESSURE **Every architectural choice in VibeVM
is evaluated against its token cost to the user.** A feature, a format, a
process step, a default — each answers: what does this make the user pay, per
session and per agent? A cheaper-by-construction design beats a
cheaper-by-discipline one (a law the machinery enforces beats a convention a
session remembers). This PROP is the roof; the mechanisms of §4 are the rooms
already built under it. @status:spec/done

@fact:COST-MODEL **The cost model the law reasons with.** Provider prompt
caching keys on the *byte-identical prefix* of the context: a cache hit reads
an order of magnitude cheaper than fresh input and cuts latency; a cache write
costs slightly more than plain input; and output tokens are the most expensive
class of all. Three consequences order the whole design space: *(a)* stable
prefixes are worth engineering for — they convert every repeat load into the
cheap class; *(b)* what mutates must sit AFTER what does not, because a single
changed byte invalidates everything downstream of it in the prefix; *(c)*
screen output is spent at the dearest rate, so chatter is the first thing to
cut (the subagent quiet clause and the `auto` interaction mode exist for
exactly this). @status:spec/done

## 3. STATIC — the cache-stable prefix {#static-prefix}

@fact:STATIC-ROLE **What STATIC is (the owner's definition, refined): the
generated static boot lane is a CACHE-STABLE PREFIX in LLM-economy terms.** It
is loaded into an agent or subagent once, at the head of its context, and from
then on stays byte-for-byte identical across sessions and spawns — so the
provider's prompt cache serves it as a hit and the user never pays full price
for the prefix twice. It is compiled, anchor-qualified, self-contained
(resolution rules ride inside it), and deliberately front-loaded: the highest
priority content reads first precisely because first is where cache stability
lives. @status:spec/done

@fact:STATIC-INVALIDATION-LAW **Invalidation is structural only.** STATIC may
change when the project's structure changes — the dependency list, dependency
versions, the snippet structure of the static lane, the materialisation format
(PROP-045 ##STATIC-FOLLOWS-THE-TARGET) — and never for small causes: a status
flip, a date, a counter, a session artifact. STATIC is a pure function of
structural inputs and NOT a function of the mutable (the adoption registry,
statuses, timestamps) — PROP-046 ##LAW-STATIC-INVARIANT is this law's
registry-side half. Regenerating with unchanged structural inputs MUST emit
identical bytes; that identity is measurable and belongs in the gates.
@status:spec/done

@fact:STATIC-PREFIX-SHARING **The multiplier: one prefix, every agent.** When
the boss and all its workers load the SAME byte-identical prefix, the cache
warmed by any one of them serves every later spawn — each subagent starts
cheap. This makes the stability requirement STRICTER than per-session
determinism: nothing per-session and nothing per-agent may enter the prefix —
no agent names, no session ids, no roles, no timestamps. Per-agent material
belongs after the prefix, in the variable tail. @status:spec/done

@fact:STATIC-POSITION-LAW **The prefix is only a prefix if nothing variable
precedes it.** Cache keying is positional: every byte BEFORE the static lane
in the loaded context must be as stable as the lane itself (the harness system
text, the instruction files' stable head), and everything variable — the
dynamic lane, session state, task text — reads strictly AFTER it. A boot
order that interleaves mutable material ahead of STATIC forfeits the whole
economy regardless of how stable STATIC itself is. @status:spec/done

@fact:STATIC-HARDENING-WAVE **The build this declaration names (a future wave,
on the owner's word).** *(i)* Strip the mutable from the compiled lane —
statuses are not compiled into STATIC at all; a fact's current status is a
point query (the registry, the dynamic lane), not prefix knowledge. *(ii)* A
byte-identity gate: regenerate twice over unchanged structural inputs and
diff — zero bytes moved, wired into the panel. *(iii)* A cache-friendliness
audit of the generated text (no dates, no unstable ordering, no absolute
paths that vary per checkout). The static-splice determinism tests are this
wave's seed. @status:spec/plan

## 4. Mechanisms already serving the law {#mechanisms}

@fact:mechanisms-lead Each of these predates this PROP; the roof names what
they share: @status:spec/done

- @fact:MECH-TREE-SHAKING **Tree-shaking loading** — the `normal` package
  format loads NOTHING a `#use` does not pull (PROP-035 §3, ##USE-ANCESTOR-RULE):
  the user pays for the contracts a session actually needs, not for a
  package's whole corpus. @status:spec/done
- @fact:MECH-DYNAMIC-CONDITIONS **The conditional dynamic lane** — a
  `when = "os:…"`-guarded INDEX entry is read only when its condition holds
  (measured live in the §5a router stand, 36/36): the inapplicable text is
  never loaded at all. @status:spec/done
- @fact:MECH-INDEX-MANIFEST **INDEX as a manifest, not a payload** — the boot
  lane names files and conditions; an agent reads on demand instead of
  swallowing the tree. @status:spec/done
- @fact:MECH-DELEGATION **Delegation-first** — bulk execution runs on cheap
  worker slots; the expensive model spends on judgment and review
  (`flow:org.vibevm.fractality/delegation-first`). @status:spec/done
- @fact:MECH-QUIET-WORKERS **The subagent quiet clause and `auto` mode** —
  output tokens are the dearest class, so workers write artifacts, not
  narration, and the central agent telegraphs
  (AGENT-MODE.toml; `##subagent-quiet-clause`). @status:spec/done
- @fact:MECH-WEAVE-DIGEST **Weave digests and sharding** — whole-corpus LLM
  loads go through `--digest` (the map form that always fits) or token-capped
  shards instead of raw concatenation (PROP-047 ##CMD-WEAVE). @status:spec/done
- @fact:MECH-POINT-QUERIES **Point queries over prefix knowledge** — the
  adoption registry answers «what is this fact's status here» as a lookup
  (`vibe facts get`), so mutable state needs no place in any loaded lane
  (PROP-046). @status:spec/done

## 5. Directions — «в дальнейшем можно сделать ещё больше» {#directions}

@fact:directions-lead Open, deliberately unscheduled; each becomes real work
only by the owner's word: @status:spec/done

- @fact:DIR-BOOT-COST-METER **A boot-cost meter** — measure a session's boot
  in tokens (prefix / dynamic / on-demand classes) so the economy is a number
  the gates can watch, not a belief; the §5a stand already knows how to drive
  live agents for measurements. @status:spec/plan
- @fact:DIR-CACHE-AWARE-ORDERING **Cache-aware ordering of the variable
  tail** — sort the dynamic lane and generated manifests by mutation
  frequency (stable-first), so even the tail's cold bytes start as late as
  possible. @status:spec/plan
- @fact:DIR-PACKET-BUDGETS **Token budgets in worker packets** — a packet
  names its expected read set and output ceiling, making a worker's spend a
  reviewed quantity like its diff. @status:spec/plan
- @fact:DIR-DEDUP-READS **Read-once discipline across a session** — the weave
  digest and the mirror views exist so a corpus is loaded once in compact form
  instead of many times in fragments; extend the same law to boss workflows.
  @status:spec/plan

## 6. Companions {#companions}

@fact:companions-list The neighbouring canon this roof spans:
[PROP-035](../modules/vibe-workspace/PROP-035-spec-compiler.md) (the compiled
static lane and tree-shaking), [PROP-045](PROP-045-xml-spec-sources.md)
(##STATIC-FOLLOWS-THE-TARGET — the lane's materialisation format),
[PROP-046](PROP-046-adoption-facts-registry.md) (##LAW-STATIC-INVARIANT — the
registry never touches the prefix), [PROP-009] (the loading model), the
delegation-first flow, and `AGENT-MODE.toml`. @status:spec/done
