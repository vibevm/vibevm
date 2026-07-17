# AI-Native Go (stack:org.vibevm.ai-native/go-ai-native-lang)

The Go projection of the AI-Native Code Discipline — and the **runnable
toolchain** that enforces it (PROP-024 code-bearing packages). Go is the
Discipline's third supported language, after Rust (the pilot) and
TypeScript.

> Campaign in flight: this package is being built end-to-end per
> `spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md` in the vibevm dev tree.
> This README is finalized at campaign close.

## What ships (target state)

- Four binaries: `go-ai-native` (umbrella: init / floor / conform /
  specmap / trace / test-gate / tripwire / health / fast-loop / codemod),
  `go-ai-native-conform`, `go-ai-native-specmap`, `go-ai-native-tcg`
  (the agentic type oracle over the consumer's gopls).
- The Go guide and cards (`spec/go/GUIDE-AI-NATIVE-GO.md`, `spec/cards/`).
- Two agent skills: `/go-ai-native-sweep`, `/go-ai-native-terraform`.
- The stdlib-only fact extractor (`tools/go-extract/`).

**Prerequisites:** go ≥ 1.24 and gopls
(`go install golang.org/x/tools/gopls@latest`) on the consumer machine.
