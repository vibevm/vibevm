#!/usr/bin/env bash
set -euo pipefail

HARNESS_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
XMLMEASURE_ROOT=${XMLMEASURE_ROOT:-/c/Users/olegc/git/v/vibevm-xmlmeasure}
XMLMEASURE_ROOT=$(realpath -m "$XMLMEASURE_ROOT")
MAIN_REPO=/c/Users/olegc/git/v/vibevm
MAIN_PACKAGES=$MAIN_REPO/packages
REGISTRY=$XMLMEASURE_ROOT/registry
SETTINGS=$XMLMEASURE_ROOT/settings

fail() {
  printf 'SETUP-ASSERT-FAIL %s\n' "$*" >&2
  exit 1
}

case "$XMLMEASURE_ROOT" in
  *vibevm-xmlmeasure*) ;;
  *) fail "XMLMEASURE_ROOT must contain vibevm-xmlmeasure" ;;
esac

if [[ -z "${VIBE_BIN:-}" ]]; then
  if [[ -f "$MAIN_REPO/target/release/vibe.exe" ]]; then
    VIBE_BIN=$MAIN_REPO/target/release/vibe.exe
  elif [[ -f "$MAIN_REPO/target/debug/vibe.exe" ]]; then
    VIBE_BIN=$MAIN_REPO/target/debug/vibe.exe
  else
    fail "vibe.exe not found in main repository release or debug target"
  fi
fi

VIBE_BIN=$(realpath -m "$VIBE_BIN")
case "$VIBE_BIN" in
  "$XMLMEASURE_ROOT"/*) fail "VIBE_BIN must be outside XMLMEASURE_ROOT" ;;
esac
[[ -f "$VIBE_BIN" ]] || fail "VIBE_BIN is not a file: $VIBE_BIN"
[[ -d "$MAIN_PACKAGES/org.vibevm.world" ]] || fail "missing mainline org.vibevm.world"
[[ -d "$MAIN_PACKAGES/org.vibevm.ai-native" ]] || fail "missing mainline org.vibevm.ai-native"
[[ -d "$MAIN_PACKAGES/org.vibevm.fractality/delegation-rules" ]] || fail "missing delegation-rules"
[[ -d "$MAIN_PACKAGES/org.vibevm.fractality/delegation-first" ]] || fail "missing delegation-first"
[[ -d "$HARNESS_DIR/probes/org.vibevm.probe" ]] || fail "missing probe registry"

rm -rf -- "$XMLMEASURE_ROOT"
mkdir -p "$REGISTRY/org.vibevm.fractality" "$SETTINGS"

copy_registry_tree() {
  local relative=$1
  tar -C "$MAIN_PACKAGES" \
    --exclude='*/target' --exclude='*/target/*' \
    --exclude='*/.git' --exclude='*/.git/*' \
    --exclude='*/node_modules' --exclude='*/node_modules/*' \
    --exclude='*/runs' --exclude='*/runs/*' \
    -cf - "$relative" | tar -C "$REGISTRY" -xf -
}

copy_registry_tree org.vibevm.world
copy_registry_tree org.vibevm.ai-native
copy_registry_tree org.vibevm.fractality/delegation-rules
copy_registry_tree org.vibevm.fractality/delegation-first
cp -R "$HARNESS_DIR/probes/org.vibevm.probe" "$REGISTRY/"

if find "$REGISTRY" -type d \( -name target -o -name .git -o -name node_modules -o -name runs \) -print -quit | grep -q .; then
  fail "excluded directory leaked into registry"
fi

TEMPLATE=$XMLMEASURE_ROOT/project-template
mkdir -p "$TEMPLATE/spec"
cat > "$TEMPLATE/vibe.toml" <<'TOML'
[project]
name = "xml-measure-polygon"
version = "0.1.0"

[requires.packages]
"flow:org.vibevm.world/redbook" = "=1.0.0"
"flow:org.vibevm.probe/router-win" = "=1.0.0"
"flow:org.vibevm.probe/router-aux" = "=1.0.0"
"flow:org.vibevm.probe/router-nix" = "=1.0.0"
TOML
cat > "$TEMPLATE/spec/PLAN.md" <<'PLAN'
# XML measure polygon

## Назначение

Одноразовый проект измеряет, как холодный агент выполняет сессионный boot.

Во время probe-прогона содержимое проекта нельзя редактировать или дополнять.
PLAN
cat > "$TEMPLATE/CLAUDE.md" <<'AGENT'
# Проект замера

Проект замера. Выполняй boot-блок ниже.
AGENT
cp "$TEMPLATE/CLAUDE.md" "$TEMPLATE/AGENTS.md"

insert_format() {
  local manifest=$1
  local format=$2
  local temporary=$manifest.tmp
  awk -v format="$format" '
    { print }
    /^name = "xml-measure-polygon"$/ { print "spec_format = \"" format "\"" }
  ' "$manifest" > "$temporary"
  mv "$temporary" "$manifest"
}

for variant in xml markdown mixed; do
  variant_dir=$XMLMEASURE_ROOT/polygon-$variant
  cp -R "$TEMPLATE" "$variant_dir"
  insert_format "$variant_dir/vibe.toml" "$variant"
  (
    cd "$variant_dir"
    VIBE_SETTINGS="$SETTINGS" "$VIBE_BIN" install --registry "$REGISTRY" --assume-yes
  )
  cp "$variant_dir/CLAUDE.md" "$variant_dir/AGENTS.md"
done

dynamic_path_suffix_ok() {
  local index=$1
  local suffix=$2
  awk -v suffix="$suffix" '
    function check_entry() {
      if (kind == "dynamic" && substr(path, length(path) - length(suffix) + 1) != suffix) bad = 1
      kind = ""
      path = ""
    }
    /^\[\[entry\]\]$/ { check_entry(); next }
    /^kind = / { value = $0; sub(/^kind = "/, "", value); sub(/"$/, "", value); kind = value }
    /^path = / { value = $0; sub(/^path = "/, "", value); sub(/"$/, "", value); path = value }
    END { check_entry(); exit bad }
  ' "$index"
}

count_dynamic_when() {
  local index=$1
  local expected=$2
  awk -v expected="$expected" '
    function check_entry() {
      if (kind == "dynamic" && condition == expected) count++
      kind = ""
      condition = ""
    }
    /^\[\[entry\]\]$/ { check_entry(); next }
    /^kind = / { value = $0; sub(/^kind = "/, "", value); sub(/"$/, "", value); kind = value }
    /^when = / { value = $0; sub(/^when = "/, "", value); sub(/"$/, "", value); condition = value }
    END { check_entry(); print count + 0 }
  ' "$index"
}

assert_variant() {
  local variant=$1
  local index=$XMLMEASURE_ROOT/polygon-$variant/spec/boot/INDEX.md
  [[ -f "$index" ]] || fail "$variant INDEX.md missing"

  local dynamic_count windows_count linux_count
  dynamic_count=$(grep -c '^kind = "dynamic"$' "$index" || true)
  windows_count=$(count_dynamic_when "$index" os:windows)
  linux_count=$(count_dynamic_when "$index" os:linux)
  (( dynamic_count >= 2 )) || fail "$variant has fewer than two dynamic entries"
  (( windows_count >= 2 )) || fail "$variant has fewer than two Windows entries"
  (( linux_count == 1 )) || fail "$variant must have exactly one Linux entry"

  case "$variant" in
    xml) dynamic_path_suffix_ok "$index" .xml || fail "xml dynamic path has a non-.xml suffix" ;;
    markdown) dynamic_path_suffix_ok "$index" .md || fail "markdown dynamic path has a non-.md suffix" ;;
  esac
  printf 'SETUP-ASSERT-OK %s\n' "$variant"
}

assert_variant xml
assert_variant markdown
assert_variant mixed
