# Research-level specs

Self-contained backgrounder documents — comparative research, threat-model analyses, prior-art surveys. Each is meant to be re-readable months after publication without referring to the original sources, and to outlast any one external project's URL stability.

When a research document identifies actionable items, those land as roadmap deltas referencing the PROP, then as their own PROP / FEAT documents under [`spec/common/`](../common/) or [`spec/modules/`](../modules/) when prioritised.

## Index

- [PROP-004: Tessl comparative research](PROP-004-tessl-comparative-research.md) — full inventory of Tessl's product surface (CLI, primitives, evaluation framework, MCP integration, registry model), gap analysis vs vibevm, and recommended roadmap deltas (M1.7 `vibe-mcp`, M1.8 `vibe review`, M1.9 PURL `describes`, M1.10 `vibe outdated`, M1.11 agent auto-detection, M2.7-M2.10 LLM-driven evaluation, M3.1 security review). Captured 2026-05-04 against Tessl CLI 0.78.0.
- [Settings-Home Consolidation & Global Registry Config — Design Plan v0.1](SETTINGS-HOME-AND-GLOBAL-REGISTRY-PLAN-v0.1.md) — verified findings + locked owner decisions for (1) a machine-global `~/.vibe/registry.toml` merged project-first into the resolver, and (2) a single `~/.vibe` settings home with a `$VIBE_SETTINGS` override, folding the legacy `~/.vibevm` and XDG `~/.config/vibe` in through one `vibe-core::settings` chokepoint; includes the `--offline` local-only refinement, the change surface, the AI-native test plan, and the phased build sequence. Captured 2026-07-20.
