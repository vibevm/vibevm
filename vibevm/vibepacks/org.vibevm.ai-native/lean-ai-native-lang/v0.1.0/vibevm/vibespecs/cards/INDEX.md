# AI-Native Lean cards {#root}

<status stage="spec" state="work" audience="user"/>

@fact:ROSTER These nine BETA cards project `spec://org.vibevm.ai-native/core-ai-native/02-EXECUTABLE-SCAFFOLDS#root`. Their shape follows `spec://org.vibevm.ai-native/core-ai-native/01-PATTERN-CARD-FORMAT#root`. Each contains recognition, tradeoffs and a bounded `card-ops` block. @status:spec/work

| Class | Card | Trigger mode | Checker boundary |
| --- | --- | --- | --- |
| A | [Generators](scaffold-a-generators.xml) | gate | General Lean source/carrier generation is WISH: the initial runner scans/checks, and no generator is commissioned. Generated text would still require ordinary checking. |
| B | [Typed builders and indexed state](scaffold-b-typed-builders.xml) | gate | Lean elaboration checks the selected source; there is no package-specific builder generator or proof that the modeled invariant matches the world. |
| C | [Checked model contracts](scaffold-c-runnable-contracts.xml) | inline | G1 checks the formal model theorem only; external observation truth and implementation correspondence remain separate premises. |
| D | [Differential and characterization experiments](scaffold-d-differential-oracle.xml) | gate | A general implementation/model differential harness is WISH in 0.1.0. Matching an old defect is not acceptance; the runner does not issue G2 or G3. |
| E | [Targeted proof loop](scaffold-e-fast-loop.xml) | gate | The targeted runner is the initial loop. Whole-project floor/conform/health and automatic card activation are WISH. |
| F | [Subject-bound diagnostics](scaffold-f-structured-diagnostics.xml) | inline | The runner must distinguish failure from successful scoped proof. A general Lean conform/SARIF system is WISH. |
| G | [Named checked examples](scaffold-g-doctests.xml) | gate | The targeted checker checks registered named examples. Automatic extraction/checking of documentation fences is WISH; prose examples are explanatory until checked as source. |
| H | [Local reference models](scaffold-h-simulators.xml) | review | The runner can check a supplied model theorem. General automatic exploration, simulation and counterexample extraction are WISH in this version. |
| I | [Scaffolded edit operations](scaffold-i-codemods.xml) | review | Lean codemods are WISH: this runner has no source-edit command. A changed proof/subject still needs a fresh check and deliberate witness invalidation. |

@fact:MANUAL-DELIVERY Select a relevant Band-3 routine manually. No automatic card trigger evaluator, delivery mechanism or complete discipline floor is claimed. @status:spec/done

@fact:PARITY-GAPS WISH means the machinery is absent, not passed. The narrower 0.1.0 scope and reasons are recorded at `spec://org.vibevm.ai-native/lean-ai-native-lang/lean/GUIDE-AI-NATIVE-LEAN#PARITY`. G1 model checking does not issue G2, G3 or G4 evidence. @status:spec/done
