# asserts — give-your-agent-the-skill

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/give-your-agent-the-skill/work/hello-vibe
===== ASSERT: vibe mcp status
Detected agents: claude, claude-desktop, opencode, codex
  → would-create mcp     claude (project) → <scratch>/PP-O1/give-your-agent-the-skill/work/hello-vibe/.mcp.json (file does not exist yet)
  → would-update mcp     claude (user) → C:/Users/olegc/.claude.json (mcpServers/vibevm differs)
  → would-update mcp     claude-desktop (user) → C:/Users/olegc/AppData/Roaming/Claude/claude_desktop_config.json (mcpServers/vibevm absent)
  → would-create mcp     cursor (project) → <scratch>/PP-O1/give-your-agent-the-skill/work/hello-vibe/.cursor/mcp.json (file does not exist yet)
  → would-create mcp     cursor (user) → C:/Users/olegc/.cursor/mcp.json (file does not exist yet)
  → would-create mcp     opencode (project) → <scratch>/PP-O1/give-your-agent-the-skill/work/hello-vibe/opencode.json (file does not exist yet)
  → would-update mcp     opencode (user) → C:/Users/olegc/.config/opencode/opencode.json (mcp/vibevm differs)
  → would-update mcp     codex (user) → C:/Users/olegc/.codex/config.toml ([mcp_servers.vibevm] absent)
  → would-create skill   claude (project) → <scratch>/PP-O1/give-your-agent-the-skill/work/hello-vibe/.claude/skills/vibevm/SKILL.md
  → would-update skill   claude (user) → C:/Users/olegc/.claude/skills/vibevm/SKILL.md
  → would-create skill   opencode (project) → <scratch>/PP-O1/give-your-agent-the-skill/work/hello-vibe/.opencode/skills/vibevm/SKILL.md
  → would-update skill   opencode (user) → C:/Users/olegc/.config/opencode/skills/vibevm/SKILL.md
  → would-create skill   codex (project) → <scratch>/PP-O1/give-your-agent-the-skill/work/hello-vibe/.agents/skills/vibevm/SKILL.md
  → would-create skill   codex (user) → C:/Users/olegc/.agents/skills/vibevm/SKILL.md
----- exit: 0
===== ASSERT: vibe skill list --quiet
(no skills declared by the project or installed packages)
----- exit: 0
```
