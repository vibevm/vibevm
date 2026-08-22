#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 --lane codex|claudez --variant xml|markdown|mixed [--model MODEL] [--effort EFFORT]" >&2
  exit 2
}

lane=
variant=
model=
effort=

while (($#)); do
  case "$1" in
    --lane) (($# >= 2)) || usage; lane=$2; shift 2 ;;
    --variant) (($# >= 2)) || usage; variant=$2; shift 2 ;;
    --model) (($# >= 2)) || usage; model=$2; shift 2 ;;
    --effort) (($# >= 2)) || usage; effort=$2; shift 2 ;;
    *) usage ;;
  esac
done

[[ "$lane" == codex || "$lane" == claudez ]] || usage
[[ "$variant" == xml || "$variant" == markdown || "$variant" == mixed ]] || usage

if [[ "$lane" == codex ]]; then
  model=${model:-gpt-5.6-sol}
  effort=${effort:-low}
else
  model=${model:-big}
  effort=${effort:-max}
fi

[[ "$model" =~ ^[A-Za-z0-9._-]+$ ]] || { echo "invalid model name" >&2; exit 2; }
[[ "$effort" =~ ^[A-Za-z0-9._-]+$ ]] || { echo "invalid effort name" >&2; exit 2; }

XMLMEASURE_ROOT=${XMLMEASURE_ROOT:-/c/Users/olegc/git/v/vibevm-xmlmeasure}
source_project=$XMLMEASURE_ROOT/polygon-$variant
[[ -f "$source_project/spec/boot/INDEX.md" ]] || {
  echo "variant is not prepared; run setup.sh first: $variant" >&2
  exit 1
}

stamp=$(date -u +%Y%m%dT%H%M%SZ)
run_id=$stamp-$lane-$model-$variant
run_dir=$XMLMEASURE_ROOT/runs/$run_id
project_dir=$run_dir/project
mkdir -p "$XMLMEASURE_ROOT/runs"
mkdir "$run_dir"
mkdir "$project_dir"
cp -R "$source_project/." "$project_dir/"
project_abs=$(cd "$project_dir" && pwd -W 2>/dev/null || pwd -P)

prompt=$(cat <<PROMPT
Ты — холодный агент в проекте $project_abs. Выполни
сессионный бут строго по CLAUDE.md этого проекта: прочитай всё, что
предписывает его boot-блок, в предписанном порядке и с предписанными
условиями. После бута ответь ровно тремя строками вида:
N: <значение или НЕДОСТУПНО> | файл: <относительный путь или -> | почему: <одна фраза>
Вопросы:
1: какой «Маяк развёртывания» объявляет flow router-win?
2: какой «Маяк развёртывания» объявляет flow router-aux?
3: какой «Маяк развёртывания» объявляет flow router-nix?
Отвечай только тем, что прочитал по правилам бута; если правила бута
предписывают пропустить источник — напиши НЕДОСТУПНО и объясни в поле
«почему». Ничего не редактируй и не создавай.
PROMPT
)

cat > "$run_dir/meta.txt" <<META
run_id=$run_id
stamp=$stamp
lane=$lane
model=$model
effort=$effort
variant=$variant
project=$project_abs
META

set +e
if [[ "$lane" == codex ]]; then
  (
    cd "$project_dir"
    CODEXRUNNER_MODEL="$model" CODEXRUNNER_EFFORT="$effort" \
      "$HOME/opt/bin/codexrunner" exec "$prompt" </dev/null
  ) >"$run_dir/transcript.log" 2>&1
  rc=$?
else
  (
    cd "$project_dir"
    "$HOME/opt/bin/claudez" -p "$prompt" \
      --output-format text \
      --allowedTools "Read" "Glob" "Grep" </dev/null
  ) >"$run_dir/transcript.log" 2>&1
  rc=$?
fi
set -e

printf 'exit_code=%s\n' "$rc" >> "$run_dir/meta.txt"
printf 'run_dir=%s\n' "$run_dir"
exit "$rc"
