#!/bin/sh
# A fake coding agent for the PROP-064 acceptance fixture. It records the
# arguments, the hook environment, the working directory, the prompt and the
# context it was given under $FAKE_AGENT_RECORD, prints one transcript line,
# and answers as $FAKE_AGENT_STATUS (ok | fail | none) with exit status
# $FAKE_AGENT_EXIT. It is started through `sh`, so it needs no execute bit.
record="${FAKE_AGENT_RECORD:?FAKE_AGENT_RECORD names the record directory}"
mkdir -p "$record"
echo call >> "$record/calls"
: > "$record/args"
for arg in "$@"; do
  printf '%s\n' "$arg" >> "$record/args"
done
: > "$record/env"
for name in VIBE_HOOK_STEP VIBE_HOOK_RUN VIBE_HOOK_PROMPT_FILE VIBE_HOOK_CONTEXT_FILE \
  VIBE_WORK VIBE_SOURCES VIBE_OUT VIBE_PACKAGE VIBE_APP_ROOT FAKE_AGENT_NAME PATH VIBE_APP_USE; do
  printf '%s=%s\n' "$name" "$(printenv "$name")" >> "$record/env"
done
pwd > "$record/cwd"
cp "$VIBE_HOOK_PROMPT_FILE" "$record/prompt.md"
cp "$VIBE_HOOK_CONTEXT_FILE" "$record/context.json"
echo "fake agent ran step $VIBE_HOOK_STEP"
status="${FAKE_AGENT_STATUS:-ok}"
if [ "$status" != none ]; then
  printf '{"protocol":"vibe-agent-hook-result/1","status":"%s","message":"fake agent answered %s","files_changed":["a.txt"]}\n' \
    "$status" "$status" > "$VIBE_HOOK_RUN/result.json"
fi
exit "${FAKE_AGENT_EXIT:-0}"
