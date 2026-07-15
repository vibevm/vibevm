# Action systems in VSCode and IntelliJ IDEA — a comparative study {#root}

**Genre:** research (comparative-research flow) — non-binding, evergreen. **Status:** COMPLETE —
part (a), both source studies (quote-first), the 14 design obligations (DO1–DO14), the two-way
gap analysis, the 12 roadmap deltas (Δ1–Δ12 → prospective PROP-039 REQs), and the predictions
check (P1–P5 CONFIRMED, P6 SUPPORTED) are all written; feeds Spec 1 behind the firewall. **Provenance:** local read-only snapshots at
`C:\Users\olegc\git\snapshot\vscode` (`microsoft/vscode`, MIT) and
`C:\Users\olegc\git\snapshot\idea` (`intellij-community`, Apache-2.0), accessed **2026-07-15**;
exact upstream commits not captured (see §8 re-fetch). **Firewall:** this document is the
**only** artifact that crosses from the source study into Spec 1 / Spec 2 / implementation
(`spec://vibevm/research/ACTION-SYSTEM-RESEARCH-PLAN#clean-room`); the sources themselves do
not. Everything here is in our words; short verbatim snippets (`file:line`) ground claims,
never bulk code. Plan: [`ACTION-SYSTEM-RESEARCH-PLAN-v0.1`](ACTION-SYSTEM-RESEARCH-PLAN-v0.1.md).

## 0. Why this study exists {#why}

We are designing a **frontend-agnostic, addressable action system** in Rust: every action a
UI can perform has a stable **address** (`action://…`), a typed parameter schema, a typed
context-enablement predicate, a mandatory human-readable name + description, and is invocable
**programmatically** — not only by a key press. On top of it sits a **Search Everywhere**
surface that searches packages, package-card fields, and all actions, and can invoke a found
action (the owner's acceptance,
`spec://vibevm/research/ACTION-SYSTEM-RESEARCH-PLAN#mandate`). VSCode and IntelliJ are the two
most mature action systems in existence; we study both clean-room — taking **ideas, never
code** — to compile the best of each and to learn from where they hurt. The through-line: this
is vibevm's own founding move — *address everything* (`spec://`) — carried from specs to
**behaviour** (`action://`).

---

## 1. Part (a) — the independent design-space map (written BEFORE the sources) {#design-space}

Written first, from first principles, to state what an action system *needs* before we
describe what two implementations *do* — so the study confirms or refutes a stated hypothesis
rather than paraphrasing incumbents (the anchoring guard). Twelve concerns; each states the
question, the design axes, and **our leaning** (the hypothesis the sources then test).

**1. Identity & addressing.** *How is an action named, made unique, versioned?* Axes: opaque
string vs structured address; flat vs namespaced; uniqueness by convention vs enforced;
rename-in-place vs tombstone/alias; version in the address vs separate. **Leaning:** a
structured, namespaced, **enforced-unique** address `action://<group>/<name>`, typed
parameters in the query; a rename is a new identity with a tombstone/alias, never silent
(ties to `qualified-naming` + `addressable-specs`).

**2. The action value.** *What is an action, as data?* Axes: behaviour-only (a command) vs a
bundle (command + presentation + menu + keybinding); a class/trait vs a data record; a live
mutable object vs an immutable resolved snapshot. **Leaning:** a first-class record —
`{ address, presentation{name, description, icon?, category?}, params, enablement, invoke →
Result }`; invocation is the primary interface; UI, keybinding, and palette are just
*invokers*; what the UI renders is a cheap immutable snapshot.

**3. Registry.** *How do actions register and resolve; what is a collision?* Axes: single map
vs override-stack; permissive-duplicate vs hard-unique; static-import vs runtime; declarative
data vs imperative code. **Leaning:** declarative registration with a typed code binding; a
collision is a **hard, deterministic error** (collision ≠ conflict, per `qualified-naming`);
the registry is **enumerable** for golden coverage; if layered override is wanted, it is one
*uniform explicit* semantics, not an accident of which door you used.

**4. Invocation.** *How is an action run?* Axes: UI-event-only vs programmatic-primary; sync vs
async; typed result vs void; cancellation; in-process vs RPC. **Leaning:**
`invoke(address, args, ctx) -> Result` is *the* interface; async + cancellation; a typed
result/error; a serializable address+params so an out-of-process frontend (web, plugin) can
drive the same core.

**5. Parameters.** *How are inputs typed, validated, supplied?* Axes: untyped varargs vs a
typed schema; positional vs named; validation opt-in vs always; how a palette elicits them.
**Leaning:** a typed, serializable **named-parameter schema** per action, validated at invoke,
suppliable by a palette / script / RPC; simple params ride in the address query.

**6. Context & enablement.** *How is enabled/visible computed?* Axes: a stringly `when` DSL vs
a typed predicate; a nullable data-context vs a typed context; UI-thread-coupled vs pure;
one axis vs two (visible vs enabled); "why disabled" introspection. **Leaning:** a **typed
context snapshot**; enablement is a **pure, fast** function → `{ visible, enabled, reason }`;
keep the two-axis model (hide vs grey-out) but typed and introspectable; no UI-thread hazard.

**7. Presentation & i18n.** *Label/description/icon, and localizable vs stable id?* Axes:
entangled vs decoupled; i18n bundle vs inline; name/description mandatory vs optional.
**Leaning:** presentation **decoupled** from the stable address; **name + description are
mandatory** (the owner's founding discipline), **searchable**, and **gate-enforced**;
i18n-ready via a label indirection (English shipped now, per PROP-037 §1.6).

**8. Discovery / Search Everywhere.** *The index, providers, matcher, ranking, grouping,
tabs?* Axes: one engine vs per-provider scorer; a contributor/provider model; match tiers;
recency/frequency; a hybrid "All" tab vs per-category tabs; async fast/slow. **Leaning:** a
**provider model** (actions, packages/tree-nodes, and every package-card field now; files/
symbols later) feeding one core index that returns ranked hits; a **match-tier ladder**
(exact → prefix → CamelCase/subsequence → substring → **word in name/description** as the
fallback lane); **recency-weighted** (VSCode's lesson: recency beats raw score for commands);
a **hybrid "All" tab + per-category tabs** (the IDEA idiom the owner named); async
fast-then-slow with cancellation. The core ranks; the frontend only renders.

**9. Keybindings & menus.** *Bindings, keymaps, conflicts, grouping?* Axes: single vs multi
keymap; chords; conflict silent vs surfaced; weight/order priority; menu group/order/when.
**Leaning:** a keymap `key → (address, args)`; a **pure resolver** returning a 3-state result
(no-match / need-more-chords / found), timers/IME pushed to the adapter; conflicts
**surfaced**, not silent; menus = group/order/when placement resolved by a deterministic sort.

**10. Extensibility.** *Third-party contribution and self-protection?* Axes: static manifest
vs runtime; override rules; a capability/permission scope. **Leaning:** declarative
contribution (a package may contribute actions); **referential integrity enforced at
registration** (a menu/keybinding naming a missing action fails loud, not at click time); a
capability/visibility scope (load-bearing once a networked web UI can invoke by address).

**11. Frontend binding.** *The seam that lets a TUI, a web app, and an IDE plugin drive one
core?* Axes: core-owns-rendering vs an adapter; a sync API vs RPC. **Leaning:** a pure Rust
core with **zero rendering dependencies**; a `Surface`/adapter trait (TUI now; web via
JSON-RPC later); registry + resolver + SE index all UI-free.

**12. Testability & introspection.** *Headless invocation, registry enumeration, telemetry?*
Axes: retrofitted vs designed-in. **Leaning:** designed-in — every action headless-invocable;
the registry enumerable so a golden asserts "every action has a name + description, resolves,
and is reachable"; a telemetry hook; the human-legibility gate runs in CI.

---

## 2. VSCode — the command/action system {#vscode}

VSCode splits the concern into **three module-level registries** — `CommandsRegistry`
(id → behaviour), `MenuRegistry` (placement + the palette table), `KeybindingsRegistry`
(bindings) — unified at contribution time by `registerAction2`, and surfaced to users by the
**Quick Access** palette. Studied files (all under `…/vscode/src/vs/`):
`platform/commands/common/commands.ts`, `platform/actions/common/actions.ts`,
`platform/actions/common/menuService.ts`, `platform/contextkey/common/contextkey.ts`,
`platform/keybinding/common/{keybindingsRegistry,keybindingResolver}.ts`,
`platform/keybinding/common/abstractKeybindingService.ts`,
`platform/quickinput/{common/quickAccess,browser/pickerQuickAccess,browser/commandsQuickAccess}.ts`,
`base/common/fuzzyScorer.ts`, `workbench/api/common/extHostCommands.ts`.

### 2.1 Identity & registration (RQ1, RQ2) {#vscode-identity}

A command is a bare `string` id with a handler; there is **no namespace type** — the dotted
form (`workbench.action.openGlobalKeybindings`) is convention only. Strikingly, the base
registry is an **override-stack, not a unique map**:

```ts
private readonly _commands = new Map<string, LinkedList<ICommand>>();   // commands.ts:69
const removeFn = commands.unshift(idOrCommand);                          // :109  newest-first
return Iterable.first(list);                                            // :134  newest wins
```

So multiple handlers may share an id; the newest shadows the rest and disposing restores the
previous — collision is a *feature* (layered override), not an error. The **`registerAction2`
door takes the opposite stance** and throws on a duplicate:

```ts
if (CommandsRegistry.getCommand(command.id)) {                          // actions.ts:726
  throw new Error(`Cannot register two commands with the same id: ${command.id}`); }
```

`MenuId` takes a third stance — interned singletons whose constructor throws on a duplicate
identifier, with `MenuId.for(string)` as the reuse hatch (`actions.ts:333,344`). **One system,
three collision policies** (permissive-stack, hard-throw, interned-singleton) — a real
inconsistency. `registerAction2` is the unifying front door: one `desc` object fans out to all
three registries —

```ts
const { f1, menu, keybinding, ...command } = action.desc;              // actions.ts:724
CommandsRegistry.registerCommand({ id: command.id, handler: (acc, ...a) => action.run(acc, ...a) });
// …appendMenuItem per menu; registerKeybindingRule per keybinding…
if (f1) { MenuRegistry.appendMenuItem(MenuId.CommandPalette, { command, when: command.precondition }); }
```

**Lesson:** the *bundle* (`registerAction2`) is almost exactly our target — a stable id ties
title/category/icon + N menu placements + N keybindings + a "put me in the palette" flag. But
the addressing is stringly-typed with **no referential integrity** between the registries: a
menu item names its command by string and nothing checks the command exists. **Copy** the
bundle and the id-uniqueness of the front door; **fix** the string addressing (typed/interned
address) and enforce referential integrity at registration.

### 2.2 Invocation & parameters (RQ3, RQ4) {#vscode-invoke}

One entry point, always async, arguments untyped:

```ts
executeCommand<R = unknown>(commandId: string, ...args: unknown[]): Promise<R | undefined>;  // commands.ts:26
type ICommandHandler<Args, R> = (accessor: ServicesAccessor, ...args: Args) => R;            // :31
```

The generic `Args` is **erased at the call site** — `executeCommand` takes `unknown[]`, so
there is no compile-time argument checking; safety is opt-in and *runtime* via metadata
constraints (`validateConstraints(args, constraints)`, `commands.ts:94`). The handler's first
parameter is a **DI `ServicesAccessor` injected by the service** — a command is therefore not a
pure function; it can reach any service. **Lesson:** we want the opposite — typed named
parameters carried end-to-end, and an action that is (as much as possible) a pure function of
`(ctx, args)`, so it is testable headless and invocable over RPC.

### 2.3 Context & enablement — the `when` language (RQ5) {#vscode-when}

`when` is a small boolean DSL (`&&`, `||`, `!`, `==`, `!=`, `<`…`>=`, `=~`, `in`) parsed by a
hand-written recursive-descent `Parser` over a `Scanner` (`contextkey.ts:204`). The factories
**normalize on construction** — constants fold immediately, `And`/`Or` arrays flatten, sort,
dedup, and distribute OR over AND (the normal form has no parens), folding `A && !A ⇒ false`:

```ts
if (expr[i].negate().equals(expr[j])) { return ContextKeyFalseExpr.INSTANCE; }  // contextkey.ts:1746
```

Evaluation reads a **flat global string keyspace** with **intentional loose equality** and
**silent-false on type mismatch**:

```ts
// Equals:  return (context.getValue(this.key) == this.value);  // :877  eqeqeq disabled
// Greater: if (typeof this.value === 'string') return false;   // :1227 non-number ⇒ false
```

Typing is essentially absent at the boundary: `RawContextKey<T>` carries a phantom `T` and a
docs-only `_info` registry (`:1988`) that is **never consulted for validation**, and
`validateWhenClauses` (`:640`) is **syntax-only**. So a misspelled key or wrong-typed value
parses cleanly and **evaluates false forever, with no warning** — the classic invisible
failure. Enablement (`precondition`) and visibility (`when`) are **two separate expressions**:
`when` hides an item, `precondition` only greys it (`actions.ts:608`). One genuinely elegant
idea is `implies` — a *sound-but-incomplete* provable-implication over sorted arrays that backs
conflict/removal decisions (`contextkey.ts:2114`). **Lesson:** keep the two-axis model
(hide vs disable) and `implies`-style reasoning, but make context keys **declared, typed
symbols** referenced by handle — turning the #1 correctness gap (stringly `when`) into
compile-time errors, and using exact equality for free.

### 2.4 Keybindings & conflicts (RQ8) {#vscode-keys}

A binding is an `IKeybindingItem` (`keybindingsRegistry.ts:14`): a decoded (possibly
multi-chord) keybinding, a `command` (or `"-command"` to remove), `commandArgs`, a `when`, and
two weights — `weight1` a tier enum (`EditorCore=0 … ExternalExtension=400`, `:62`), `weight2`
the chord index. Winner selection is **"last match wins"**: sorted ascending by weight, then
iterate matches **from the end** and return the first whose `when` holds. The resolver returns
a clean **3-state result**:

```ts
const enum ResultKind { NoMatchingKb, MoreChordsNeeded, KbFound }   // keybindingResolver.ts:11
```

Conflicts are mostly resolved **silently** — a shorter binding shadows a longer one only when
its `when` *entirely includes* the other's (`whenIsEntirelyIncluded` → `implies`,
`:209,:253`); there is **no user-facing "these two conflict" diagnostic**, only an opt-in trace
stream. The stateful **chord machine lives in the frontend service**, not the resolver — three
timers (5 s idle, 500 ms poll, 300 ms single-modifier) + IME disable + status messages
(`abstractKeybindingService.ts:159–277`). **Lesson:** **copy the 3-state result** (it cleanly
separates "nothing", "keep listening", "fire" and keeps chord *policy* out of the pure
resolver); **surface** conflicts instead of resolving them silently; keep timers/IME in the
adapter.

### 2.5 Menus & grouping (RQ9) {#vscode-menus}

`MenuId → LinkedList<IMenuItem>`; each item has `group` (string, `'navigation'` privileged),
`order` (number), and `when`. Resolution filters by `contextMatchesRules(item.when)` and
materializes, then a **total sort** group → order → title (`menuService.ts:318`). Two nice
properties: the Command Palette is a **menu that implicitly enumerates every command** (free
discoverability — exclusion is the exception, `menuService.ts`/`actions.ts:530–554`), and
menus are **precisely reactive** — each records exactly which context keys its items depend on
and rebuilds only when `affectsSome` of them changes:

```ts
const isStructuralChange = e.affectsSome(this._menuInfo.structureContextKeys);   // menuService.ts:431
```

**Lesson:** **copy** discoverability-by-default (every action is Search-Everywhere-visible
unless it opts out) and **key-scoped reactivity** (cheap live re-resolution of enablement).

### 2.6 The palette / Quick Access — VSCode's "Search Everywhere" (RQ7) {#vscode-palette}

Providers register against a **prefix**; resolution is **longest-prefix-`startsWith` wins**
with a single default provider (bare = files/"anything", `>` = commands, `@` = symbols, …):

```ts
providers.sort((a, b) => b.prefix.length - a.prefix.length);   // quickAccess.ts:244 → 'ext install' beats 'ext'
```

The generic `PickerQuickAccessProvider` **turns off the list widget's own matching/sorting**
(`matchOnLabel = … = sortByLabel = false`) and takes over: on each keystroke it **cancels the
prior run** and chains a child cancellation token, then calls `_getPicks`, which may return
sync picks, a Promise, or **`FastAndSlowPicks`** (instant local picks + an awaited
`additionalPicks`, with a `mergeDelay` to suppress flicker, `pickerQuickAccess.ts:107,158`).

Crucially, the **command palette does not use `fuzzyScorer.ts`** — it runs an **escalation
ladder**: (1) a word filter `or(matchesBaseContiguousSubString, matchesWords)` over label +
alias, whose first hit yields the highlight ranges; (2) exact command-id equality; (3) if
`filter.length ≥ 3` and nothing matched, a **TF-IDF** natural-language pass (threshold 0.5,
top 5); (4) an optional **AI** pass as further `additionalPicks` behind a 200 ms debounce.
Ranking then **sinks TF-IDF to the bottom and orders by MRU recency** — a static
`CommandsHistory` LRU keyed by a monotonically increasing counter, persisted only at shutdown:

```ts
if (commandACounter && commandBCounter) return commandACounter > commandBCounter ? -1 : 1;  // commandsQuickAccess.ts:168 (more recent first)
```

Grouping is done by splicing `type:'separator'` rows ("recently used" / "similar commands" /
"commonly used" / "other commands"). The *other* engine, `fuzzyScorer.ts`, powers file/symbol
search — a per-query×target **DP matrix** with additive bonuses (start-of-word **+8**,
consecutive-run `min(seq,3)*6+…`, camelCase +2, after-separator +4/5), ordered thresholds
(`PATH_IDENTITY 1<<18 > LABEL_PREFIX 1<<17 > LABEL 1<<16`), a short-label boost, multi-term
AND on spaces, and `"…"` for exact/contiguous (`fuzzyScorer.ts:170–222,492,865`).

**Lessons (these shape our SE directly):**
- **COPY** longest-prefix routing to multiplex modes onto one input box — but expose it as
  **tabs** (the IDEA idiom the owner wants) as well as prefixes.
- **COPY** cancel-previous-on-keystroke via **child cancellation tokens**, and
  **`FastAndSlowPicks` + `mergeDelay`** for async providers (instant local, streamed slow).
- **COPY** the **explicit escalation ladder** (word → exact → fuzzy/TF-IDF → optional smart)
  and, above all, **recency weighting** — "recency beats score" for commands is the single
  biggest felt-quality lesson.
- **AVOID** two non-shared match engines with **highlights computed by whichever matcher
  fired**: prefer **one scorer that emits both a score and the match ranges** (as
  `fuzzyScorer` does), so highlights always explain the ranking.

### 2.7 Cross-process invocation — the RPC surface (RQ3-rpc, RQ11) {#vscode-rpc}

The extension-host `executeCommand` keeps **local commands in-process with zero serialization**
(any arg types), and for remote ones **marshals args and revives the result**:

```ts
const result = await this.#proxy.$executeCommand(id, hasBuffers ? new SerializableObjectWithBuffers(toArgs) : toArgs, retry);
return revive<any>(result);                                    // extHostCommands.ts:216
```

Two patterns worth stealing for a future web/plugin adapter: a **delegating-command cache** —
for un-serializable args, ship a synthetic handle and keep the payload local
(`delegatingCommandId = __vsc<uuid>`, `:396`) — and a **retry-on-`$executeCommand:retry`** to
tolerate registration races across processes (`:221`). Cancellation tokens thread the whole
chain; errors are wrapped with the offending command id + source and suppressed for
cancellations. **Lesson:** design the address+params to be serializable from day one, with a
handle escape hatch for values that are not.

---

## 3. IntelliJ IDEA — the action system {#idea}

IntelliJ centralises actions in one app-scoped `ActionManager` (a flat `HashMap<String, AnAction>`
sized 5000), **separates behaviour** (`AnAction` subclass) **from identity/presentation/placement/
shortcuts** (declared in `plugin.xml`), assembles input through a `DataContext` of
typed-by-convention keys, and surfaces discovery through **Search Everywhere** — a
contributor/provider model with a hybrid "All" tab. Studied under `…/idea/platform/` (note: much
of the impl is **Kotlin** now, and the SE API has been refactored — the classic
`SearchEverywhereContributor` moved to `lang-api` and is `@Deprecated` in favour of a new split
frontend/backend `com.intellij.platform.searchEverywhere.SeItemsProvider`; we studied the mature,
battle-tested classic classes whose UX we replicate): `editor-ui-api/…/actionSystem/{AnAction,
AnActionEvent,Presentation,ActionGroup,ActionUpdateThread,ActionManager}.java`,
`platform-impl/…/actionSystem/impl/ActionManager*.kt`, `core-ui/…/actionSystem/{DataContext.java,
DataKey.kt}`, `platform-impl/…/keymap/impl/{KeymapImpl,KeymapManagerImpl,IdeKeyEventDispatcher}.kt`,
`lang-impl/…/ide/actions/searcheverywhere/*`, `lang-impl/…/ide/util/gotoByName/GotoActionModel.java`.

### 3.1 Identity & registration (RQ1, RQ2) {#idea-identity}

The address is a flat `String` id in one app-global map; the id is the XML `id` attribute, else
the class's short name (`obtainActionId`, `ActionManagerXmlSupport.kt:262`). Uniqueness is
enforced at **registration time only, by logging an error and dropping the loser** — never
thrown, never a load failure. The registry keeps a careful bidirectional invariant:

```kotlin
// "bind action->id before publishing id->action so a failed duplicate cannot leave a findable id"
val existingByAction = state.putActionId(action, actionId)      // ActionManagerRegistration.kt:424
if (!addToMap(actionId, existing, action, projectType, registrar)) { state.removeActionId(action); reportActionIdCollision(...) }
```

Two nuances stand out. A repeated id is **not always** a collision — `addToMap` can fold multiple
implementations of one id into a `ChameleonAction` keyed by `ProjectType` (one address fronting N
impls, `ActionManagerXmlSupport.kt:426`). And XML actions register as a **lazy `ActionStub`** — the
behaviour class is not loaded until first `getAction`, so **address + presentation + placement +
shortcuts must be fully knowable without the class** (`ActionPluginRegistrar.kt:186`); that
separation is exactly the "stable address, discoverable, resolve on demand" model we want.
Contribution is two-path: declarative `<action>`/`<group>` XML (identity + presentation +
placement + shortcuts, with text/description from the attribute *or* a resource bundle keyed
`action.<id>.text`/`.description` — i18n by id-convention) and programmatic
`registerAction(id, action, pluginId)`. **Lesson:** **copy** the behaviour/identity/presentation
split + lazy-stub + bind-before-publish-and-rollback; **avoid** the flat stringly namespace and
log-and-drop collision — we want a structured/typed address and **load-time** uniqueness (fail
fast, not silently drop the loser).

### 3.2 Invocation & parameters (RQ3, RQ4) {#idea-invoke}

`actionPerformed(AnActionEvent)` is abstract, `void`, `@RequiresEdt`, and "must not be called
directly" (`AnAction.java:434`); programmatic invocation goes through wrappers —
`ActionUtil.performAction(action, event): AnActionResult` (the result the void method can't return
is recovered **out of band**), or `ActionManager.tryToExecute(...): ActionCallback` (async).
Tellingly, `getInputEvent()` is **null precisely when invoked programmatically / from Search
Everywhere / tests** (`AnActionEvent.java:164`) — so programmatic invocation is a first-class path,
just a retrofitted one. Parameters are **not typed method arguments**: the action pulls values out
of one `DataContext` by key, and the "typed" key is a phantom:

```java
default @Nullable <T> T getData(@NotNull DataKey<T> key) { return (T)getData(key.getName()); } // DataContext.java:65
```

`DataKey<T>` is globally interned by `name`, the `<T>` erased at the boundary, the cast unchecked —
two callers creating the same name with different `T` silently share one key (`DataKey.kt:33`).
Values are populated from focus context, not passed by the caller, so an action's real parameter
list is implicit (whichever keys its code reads, each `T?`). **Lesson:** make programmatic
invocation **primary** with **typed named parameters**; make the result **first-class** (not a
wrapper side-channel); in Rust make context keys genuinely typed (a typemap keyed by `TypeId`, or a
key trait carrying `type Value`) so same-name/different-type is impossible and no cast is needed.

### 3.3 Context & enablement — update() and the threading hazard (RQ5) {#idea-enablement}

`update(AnActionEvent)` mutates `event.getPresentation()` via `setEnabled`/`setVisible`; it "can be
called twice a second" and must be fast and side-effect-free (`AnAction.java:339`). Threading is
declared per-action, and the default is the dangerous one:

```
ActionUpdateThread { BGT, EDT }   // BGT is documented "preferred"; EDT is the actual default
// EDT: may touch Swing, must NOT touch PSI/VFS; slow work here freezes menus/popups
```

`update()` is also **advisory** — it "is not guaranteed to be called before `actionPerformed`,
which MUST re-validate the context itself" (`AnAction.java:366`). The context is two-layer: a fast
UI snapshot (walk the component hierarchy, deeper overrides shallower) plus slow background rules
that derive keys from other keys (recursion-guarded, `DataManagerImpl.kt:77`); there is a hidden
dual-null (`null` = absent vs an `EXPLICIT_NULL` sentinel = "explicitly none, stop resolving") and
**no per-context introspection** — only a process-global `DataKey.allKeys()`, so "which keys can
this context answer?" is discoverable only by probing each for null. **Lesson:** make enablement a
**pure, fast** function over a **typed context snapshot**, off the UI thread *by construction* (no
EDT/BGT hazard); let an action **declare** the keys it requires (so enablement is *derived* from
key presence) and let a context **enumerate** what it provides — both are missing in IntelliJ and
are exactly our differentiators; model `enum { Absent, ExplicitNone, Value(T) }`, never a smuggled
sentinel.

### 3.4 Keymaps, shortcuts & conflicts (RQ8) {#idea-keys}

A `Keymap` is a named scheme with a **parent chain**; `KeymapImpl` stores
`actionId → List<Shortcut>` plus lazily-cached **reverse indexes** `keystroke → [actionIds]`,
wiped on any mutation. One keystroke maps to a **list** of action ids (ambiguity is first-class),
parent-merged with child-overrides, in registration order. `KeyboardShortcut` holds one or two
`KeyStroke`s (the second is the chord); `startsWith` is the prefix test. Dispatch is a hand-rolled
FSM (`IdeKeyEventDispatcher`): it searches **local component shortcuts first**, the global keymap
only as fallback (local shadows global), and resolves ambiguity by **running `update()` and taking
the first enabled**:

```kotlin
if (event == null || !presentation.isEnabled) { continue }; return UpdateResult(action, event, ...) // first enabled wins
```

A two-stroke shortcut enters an explicit `STATE_WAIT_FOR_SECOND_KEYSTROKE` with a 2 s registry
timeout. **Conflicts are not prevented at bind time** — the model tolerates N actions on one
keystroke and resolves late; explicit surfacing is an *advisory* settings-time
`Keymap.getConflicts(actionId, shortcut)` (`KeymapImpl.kt:771`). **Lesson:** **copy** the reverse
index + parent/child inheritance + ambiguity-as-list-resolved-by-enablement + advisory
`getConflicts` + the chord FSM-with-timeout, and combine it with VSCode's pure 3-state
`ResultKind` resolver (§2.4) with the timers kept in the adapter.

### 3.5 Grouping (RQ9) {#idea-grouping}

`ActionGroup extends AnAction` (the composite pattern — a group *is* an action), with
`getChildren(event)` computed dynamically; `DefaultActionGroup` is the static-list impl with
`add(action, Constraints{anchor: FIRST|LAST|BEFORE|AFTER, relativeTo: id})`. Cross-plugin
composition is **inverted**: a child declares `<add-to-group group-id= anchor= relative-to-action=>`
and the parent is resolved by id at load (must be a `DefaultActionGroup`). A `popup` flag chooses
submenu vs inlined; a `searchable` flag drives Search-Everywhere inclusion. **Lesson:** **copy**
placement-inversion — children attach to parents by address — for frontend-agnostic grouping.

### 3.6 Search Everywhere — the contributor model, the hybrid "All" tab, ranking & invocation (RQ7) {#idea-se}

This is the centrepiece for our acceptance — study it as the reference design. The provider
interface (`SearchEverywhereContributor<Item>`, now in `lang-api`):

```java
@NotNull String getSearchProviderId();  @NotNull @Nls String getGroupName();
int getSortWeight();                                     // orders CONTRIBUTORS (tabs/groups), NOT elements
default boolean isShownInSeparateTab() { return false; }
void fetchElements(String pattern, ProgressIndicator, Processor<? super Item> consumer);   // stream; false = stop
boolean processSelectedItem(Item selected, int modifiers, String searchText);              // return: close the popup?
@NotNull ListCellRenderer<? super Item> getElementsRenderer();
```

A richer `WeightedSearchEverywhereContributor.fetchWeightedElements(...)` emits
`FoundItemDescriptor{item, weight}` so a contributor sets **each element's own priority** (the
Actions provider uses this). Providers register via a `SearchEverywhereContributorFactory`
extension point and are instantiated **once per window open** (they are `Disposable`).

**The tab strip is built *from* the contributors** (`SearchEverywhereHeader.java:208`): sort by
`getSortWeight`; **if there is more than one contributor, prepend an "All" tab**; then each
contributor whose `isShownInSeparateTab()` is true gets its own `SETab{id, name, contributors[],
actions, filter}`. The **"All" tab additionally owns a persistent checkbox-filter** over the
providers (hide categories from All without leaving it) — a second control axis. Tabs cycle with
Tab / Shift-Tab.

**How "All" runs everything and merges** (`SearchEverywhereUI` → `MixedResultsSearcher`): build a
`Map<contributor, limit>` —

```java
boolean isAllTab = selectedTab.getID().equals(ALL_CONTRIBUTORS_GROUP_ID);   // SearchEverywhereUI.java:929
int limit = contributors.size() > 1 ? 15 /*MULTIPLE*/ : 30 /*SINGLE*/;      // per-tab element caps :212
```

The searcher spawns **one task per contributor** into a shared accumulator with a **bounded
per-contributor section** (back-pressure parks a provider that overflows its cap so it cannot flood
the list). Each item becomes `FoundElementInfo{element, priority, contributor}` (ML-weighted if a
`SearchEverywhereMlService` is present), is run through a **cross-provider equality dedup** that
keeps the **higher-priority** twin, and is flushed to the UI in **~100 ms batches**. Final order:

```java
COMPARATOR = (a,b) -> priority.compareTo() ≠0 ? that : -compare(a.contributor.sortWeight, b.contributor.sortWeight)
// i.e. priority DESC, ties broken by contributor sortWeight DESC   (SearchEverywhereFoundElementInfo.java:74)
```

**The crucial hybrid-list insight:** element priority dominates; contributor weight is only a
tiebreak — so **every provider MUST emit priorities on one shared comparable scale**, or one
category drowns another. **Matching** is a `MinusculeMatcher` (camel-hump/subsequence,
`preferringStartMatches`); actions additionally match **name > startsWith > contains** (degrees
3/2/1) plus **synonyms, description (word-prefix), and group-path** (`GotoActionModel.java:324,523`),
with a type-weight ordering (available action 0 < toggle-option 1 < *unavailable* action 2 <
setting 3). An optional CatBoost **ML reorder** runs over the whole (un-frozen) list, but with a
hand-coded **exact-match floor** (0.9/0.99 bands) so exact hits always outrank learned scores;
recency/frequency enter as ML features + an empty pattern shows **recents**. Two stability tricks:
**freeze-on-"more"** (a per-provider synthetic "more…" row re-queries just that provider and
**freezes the rows above** so async/ML updates don't reshuffle under the cursor) and pervasive
**100 ms throttling** + cancel-on-restart per keystroke.

**Invocation — where a found action is run** (the acceptance's core):

```kotlin
// ActionSearchEverywhereContributor.processSelectedItem (:173)
if (modifiers == ALT_DOWN_MASK) { showAssignShortcutDialog(...); return true }
if (selected is BooleanOptionDescription) { selected.setOptionState(!on); return false }  // toggle in place, keep open
GotoActionAction.openOptionOrPerformAction(selected, text, project, ...)                   // Enter → PERFORM the action
return !inplaceChange                                                                       // perform → close; toggle → stay
```

Shortcuts render right-aligned (`KeymapUtil.getActiveKeymapShortcuts(actionId)`); disabled actions
are hidden unless an "include disabled" checkbox is on, and render greyed. Actions are fetched by a
**staged pipeline** (`ActionAsyncProvider`): abbreviations → matched actions/stubs → unmatched
stubs → top-hits → intentions → options/settings, with priority-ordered concurrency surfacing the
best ~100 of ~3000 fast.

**The reimplementation recipe (from the study, precise enough to port to a TUI):**
1. **Model:** N providers, each `{id, groupName, sortWeight, isShownInSeparateTab}`. Tabs =
   `[All]` (only if N>1, owning a persisted category checkbox-filter) + one tab per opt-in provider,
   ordered by `sortWeight`.
2. **Per keystroke (debounced ~100 ms):** cancel in-flight; active set = one provider (category
   tab) or all filter-enabled (All); per-provider cap 30 (single) / 15 (All); start one async
   search each against the shared pattern.
3. **Merge:** wrap `{element, weight, provider}`, cross-provider dedup (keep higher weight), enforce
   the per-provider cap (park overflow); push survivors in ~100 ms batches.
4. **Order:** one flat list, **weight DESC then provider sortWeight DESC**; in All draw a **group
   header** before each provider's first row (none in single tabs); optional ML reorder with an
   exact-match floor.
5. **"more…":** a trailing synthetic row per capped provider re-queries it with `size+cap` and
   **freezes** rows above.
6. **Invoke:** Enter/click → owning provider's `processSelectedItem(element, modifiers, text)`;
   return "close" dismisses, else keep open (in-place toggles). Modifiers thread through.
7. **Empty pattern:** providers that support it (actions → recents) show suggestions; the window
   opens short and expands once a pattern is typed.

**Lessons for our TUI Search Everywhere (these directly drive Spec 2 + the implementation):**
- **COPY** the provider model — our providers are **Packages-by-name**, **Package-card-fields**
  (every field of the detail card indexed), and **Actions** (invocable in place), with headroom for
  more (files, settings). Each emits `{element, priority, group, render-descriptor}` + a
  `process_selected`.
- **COPY** the **hybrid "All" + per-category tabs** built from providers, with the All-tab category
  filter — this *is* the owner's named UX.
- **COPY** the make-or-break rule (**priorities on one shared scale**), cross-provider
  **dedup-keeps-higher**, the **exact-match floor**, and **recency**.
- **COPY** **freeze-on-"more"**, debounce/throttle, cancel-on-restart, and
  **`process_selected → close?`** (Enter performs an action; a toggle keeps the popup open) — found
  actions are invoked right here, satisfying the acceptance.
- **ADAPT for a TUI:** replace per-provider Swing renderers with **one renderer over a normalized
  row descriptor** `{icon?, primary, secondary/right (e.g. a keybinding), group, enabled, kind}` so
  every category looks uniform; replace threads + `Condition`s with **round-robin draining of
  per-provider bounded queues** in the single-threaded TUI loop.

---

## 4. The complaint / failure-mode catalogue → design obligations (part c) {#pain}

Every complaint below is grounded in the two sources (and matches well-known public pain); each
becomes a numbered **design obligation (DO)** the Spec-1 contract must answer. This is the
study's checklist against the new design.

| # | The failure mode (where) | The obligation for our system |
|---|---|---|
| **DO1** | Flat stringly ids; VSCode has **three inconsistent** uniqueness policies (permissive stack vs throw vs interned), IntelliJ **log-and-drops** the loser; renames silently break keymaps/macros | A **structured, namespaced, enforced-unique, versioned** address; a rename is a new identity + tombstone/alias, never silent |
| **DO2** | No **referential integrity**: a VSCode menu/keybinding names its command by string and nothing checks it exists | Registration **validates every reference** (menu/keymap → action) and fails loud, not at click time |
| **DO3** | Untyped parameters: VSCode `...args: unknown[]` (generics erased), IntelliJ phantom `DataKey<T>` unchecked cast | A **typed, serializable, named parameter schema**, validated at invoke |
| **DO4** | Enablement is fragile: VSCode stringly `when` **evaluates false forever on a typo**; IntelliJ `update()` **EDT/BGT freeze**, nullable `DataContext`, advisory (may be skipped), no per-context introspection | Enablement is a **pure, fast** function over a **typed context snapshot**, off the UI thread by construction; actions **declare** required keys; the context is **enumerable**; "why disabled" is introspectable |
| **DO5** | Presentation is optional/entangled with identity; no legibility guarantee | **Mandatory human-readable name + description**, **searchable** (the fallback lane), **gate-enforced** (the owner's founding discipline) |
| **DO6** | Discoverability is opt-in-ish and uneven | **Discoverable by default** — every action is Search-Everywhere-visible unless it opts out |
| **DO7** | VSCode computes **highlights with one matcher and ranking with another** (word-filter vs TF-IDF), so highlights can contradict the order | **One scorer emits both** the score and the match ranges |
| **DO8** | Raw match quality alone feels wrong — VSCode: "recency beats score" for commands; IntelliJ: ML + `statistician` frequency | **Recency/frequency weighting** in ranking, with an **exact-match floor** so the obvious hit stays on top |
| **DO9** | A hybrid multi-source search is hard to get right (priority scale, flooding, reshuffle) | The **provider model + hybrid "All"/per-category tabs**, a **shared priority scale**, **dedup-keeps-higher**, per-provider caps, **freeze-on-"more"**, debounce/cancel (§3.6) |
| **DO10** | Programmatic invocation is **retrofitted**; results recovered out-of-band (IntelliJ) or `any` (VSCode) | `invoke(address, args, ctx) -> Result` is **primary**, typed result/error, async + cancellation, **RPC-ready** (serializable address+params, a delegating-handle for un-serializable values) |
| **DO11** | Keybinding conflicts resolved **silently** (VSCode) or only surfaced by an **advisory** settings query (IntelliJ) | Conflicts are **surfaced**; the resolver is a pure 3-state function, chord timers live in the adapter |
| **DO12** | Both cores are **UI-toolkit-coupled** (DOM / Swing focus, renderers, dispatch timers) | A **pure, render-dep-free core** + a `Surface` adapter seam (TUI now; web/plugin later) |
| **DO13** | Testing & headless invocation were **retrofitted** | Designed-in: the registry is **enumerable** (golden coverage), every action **headless-invocable** |
| **DO14** | **No capability/permission model** — any code invokes any action (fine for a trusted desktop IDE, a liability for a networked web UI) | A **capability/visibility scope** on invocation |

## 5. Two-way gap analysis {#gaps}

**Where a fresh Rust design can LEAD the incumbents** (their historical baggage, our clean slate):
typed addresses + typed params + a **genuinely** typed context (both are stringly at these
boundaries); **load-time** uniqueness + referential integrity (both are late/silent);
**pure, off-thread enablement** — sidestepping IntelliJ's single biggest pain (EDT/BGT freezes)
by construction; **one unified scorer** (score + ranges) instead of VSCode's two-engine mismatch;
a **capability model** (neither has one); and the **mandatory-legibility discipline with a CI
gate** (neither enforces name/description) — plus, for a TUI, **one normalized renderer** across
all categories instead of per-provider renderers.

**Where the incumbents LEAD us** (mature, hard-won, do not naively reinvent): the **depth of the
SE UX** — `freeze-on-"more"`, ML/`statistician` ranking, staged pipelines, per-provider
back-pressure (we copy the *concepts*, implemented simply); VSCode's **precise key-scoped
reactivity** and `FastAndSlowPicks` progressive async; IntelliJ's **parent/child keymap
inheritance** and chord FSM; both have battle-tested **edge handling** (IME, AltGr, focus
walking) we can scope out but should not underestimate; and **ecosystem maturity** (contribution
points, marketplaces) — we start deliberately smaller (essential-first, §0).

## 6. Roadmap deltas → Spec-1 REQ targets {#deltas}

Each delta is an actionable proposal for Spec 1 (PROP-039), naming its prospective REQ home. The
study **proposes**; ratification happens in the contract (spec-genres). Anchors are indicative.

| Δ | Delta | Answers | → prospective PROP-039 REQ |
|---|---|---|---|
| **Δ1** | The `action://<group>/<name>[?params]` address grammar; enforced-unique; tombstone/alias on rename | DO1 | `#addressing` |
| **Δ2** | The **Action value** — `{ address, presentation, params, enablement, invoke → Result }`; resolved views are immutable snapshots | RQ2 | `#action-value` |
| **Δ3** | The **collision-erroring registry** — declarative + typed binding, referential-integrity checks, enumerable | DO1, DO2, DO13 | `#registry` |
| **Δ4** | The **typed parameter schema** (named, serializable, validated) | DO3 | `#parameters` |
| **Δ5** | The **typed context** + **pure enablement** + declared requirements + "why disabled" + enumerable context | DO4 | `#context` `#enablement` |
| **Δ6** | **Presentation** + the **human-legibility discipline** (mandatory name+description, searchable, `conform` gate) | DO5 | `#presentation` `#legibility-gate` |
| **Δ7** | The **Search-Everywhere engine** — provider model; hybrid "All"+category tabs; shared priority scale; match-tier ladder with the **name/description fallback lane**; dedup-keeps-higher; recency + exact-match floor; freeze-on-"more"; debounce/cancel; **one normalized row renderer**; `process_selected → close?` (actions invoked in place) | DO6–DO9 | `#search-everywhere` |
| **Δ8** | **Invocation** — `invoke()` primary; typed result; async + cancellation; RPC-ready serialization + delegating-handle | DO10 | `#invocation` |
| **Δ9** | **Keymap** — pure 3-state resolver; parent/child inheritance; ambiguity-as-list resolved by enablement; chords/timers in the adapter; **surfaced** conflicts | DO11 | `#keymap` |
| **Δ10** | **Frontend-agnostic core** + the `Surface` adapter seam (zero render deps) | DO12 | `#frontend-seam` |
| **Δ11** | A **capability/visibility scope** on invocation | DO14 | `#capabilities` |
| **Δ12** | **Testability** — enumerable-registry golden + headless invocation | DO13 | `#testability` |

## 7. Predictions check (P1–P6) {#predictions}

- **P1** (identity separate from menu/keybinding) — **CONFIRMED.** VSCode keeps three registries
  (`registerAction2` only bundles them at contribution time); IntelliJ separates behaviour
  (`AnAction`), grouping (`ActionGroup`/`<add-to-group>`), and binding (`Keymap`) into distinct
  graphs. A naïve single-object model is not what either does — but a single *declaration* that
  fans out (VSCode's `registerAction2`) is the ergonomic sweet spot to copy.
- **P2** (no enforced namespaced/versioned ids) — **CONFIRMED.** Both use flat stringly ids
  (dotted by convention); VSCode's uniqueness is inconsistent across three doors, IntelliJ
  log-and-drops; **neither namespaces or versions the id.** Pillar 1 is a genuine improvement.
- **P3** (SE uses a provider abstraction that generalises beyond actions) — **CONFIRMED.**
  IntelliJ's `SearchEverywhereContributor<Item>` is exactly a general provider (actions, files,
  classes, symbols, settings…); VSCode's Quick Access providers-by-prefix are the same idea.
  Validates the provider-model SE (pillar 7 / Δ7).
- **P4** (untyped command args) — **CONFIRMED.** VSCode `executeCommand(...args: unknown[])`
  (generics erased, opt-in runtime constraints only); IntelliJ `getData((T)key.getName())`
  unchecked cast. Typed params (Δ4) fill a real gap.
- **P5** (dominant complaint classes) — **CONFIRMED.** IntelliJ: `update()` **EDT/BGT threading**
  + advisory update + nullable `DataContext` (the documented, burned-in pain — the header of
  `DataContext` literally forbids implementing it "since asynchronous action update"). VSCode:
  stringly `when` **silent-false** + the two-engine palette + discoverability. Both map straight
  onto DO4/DO7.
- **P6** (a frontend-agnostic core with zero UI deps is feasible) — **SUPPORTED (design-level).**
  Every core concern — address, registry, typed context, enablement, match/rank, resolver — is
  expressible with no UI type; both incumbents entangle a UI toolkit only at the **renderer** and
  the **dispatch-timer/focus** edges, which our `Surface` adapter isolates (Δ10). Final proof is a
  Spec-1 design spike, but the evidence is strong.

---

## 8. Re-fetch / provenance {#refetch}

| Subject | Upstream | Snapshot root | Accessed | Commit |
|---|---|---|---|---|
| VSCode | `microsoft/vscode` (MIT) | `C:\Users\olegc\git\snapshot\vscode` | 2026-07-15 | _not captured_ |
| IntelliJ Community | `JetBrains/intellij-community` (Apache-2.0) | `C:\Users\olegc\git\snapshot\idea` | 2026-07-15 | _not captured_ |

Clean-room posture binds regardless of licence: inspiration only, never a code source
(`spec://vibevm/research/ACTION-SYSTEM-RESEARCH-PLAN#clean-room`).
