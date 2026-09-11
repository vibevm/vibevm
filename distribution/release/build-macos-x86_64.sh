#!/usr/bin/env sh
set +x
set -eu

usage() {
    printf '%s\n' \
        'usage: build-macos-x86_64.sh [--upload] [--checks] [--tests] [--self-check]'
}

upload=0
checks=0
tests=0
self_check=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --upload) upload=1 ;;
        --checks) checks=1 ;;
        --tests) tests=1 ;;
        --self-check) self_check=1 ;;
        -h|--help) usage; exit 0 ;;
        *) printf 'build-macos-x86_64.sh: unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
    shift
done

detected_os=$(uname -s 2>/dev/null || printf unknown)
detected_arch=$(uname -m 2>/dev/null || printf unknown)
if [ "$detected_os" != Darwin ] || [ "$detected_arch" != x86_64 ]; then
    printf 'build-macos-x86_64.sh: requires macOS Intel (x86_64); detected OS=%s architecture=%s\n' \
        "$detected_os" "$detected_arch" >&2
    exit 2
fi
if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' \
        'build-macos-x86_64.sh: cargo was not found on PATH; install the repository Rust toolchain first.' >&2
    exit 127
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_dir/../.." && pwd)
github_token_set=${VIBEVM_PUBLISH_TOKEN_GITHUB+x}
github_token_value=${VIBEVM_PUBLISH_TOKEN_GITHUB-}
legacy_token_set=${VIBEVM_PUBLISH_TOKEN+x}
legacy_token_value=${VIBEVM_PUBLISH_TOKEN-}
git_config_count=${GIT_CONFIG_COUNT-0}
case $git_config_count in ''|*[!0-9]*) git_config_count=0 ;; esac
if [ "${#git_config_count}" -gt 3 ] || [ "$git_config_count" -gt 128 ]; then
    git_config_count=128
fi
git_config_index=0
while [ "$git_config_index" -lt "$git_config_count" ]; do
    unset "GIT_CONFIG_KEY_$git_config_index" "GIT_CONFIG_VALUE_$git_config_index"
    git_config_index=$((git_config_index + 1))
done
unset VIBEVM_PUBLISH_TOKEN VIBEVM_PUBLISH_TOKEN_GITHUB VIBEVM_PUBLISH_TOKEN_GITVERSE \
    VIBEVM_PUBLISH_TOKEN_GITLAB GITHUB_TOKEN GH_TOKEN GITHUB_ENTERPRISE_TOKEN \
    GH_ENTERPRISE_TOKEN GITHUB_PAT ACTIONS_ID_TOKEN_REQUEST_TOKEN ACTIONS_RUNTIME_TOKEN \
    GIT_CONFIG_COUNT GIT_ASKPASS SSH_ASKPASS SSH_AUTH_SOCK SSH_AGENT_PID GIT_SSH GIT_SSH_COMMAND

case ${CARGO_TARGET_DIR-} in
    '') xtask_target_dir=$repository_root/target ;;
    /*) xtask_target_dir=$CARGO_TARGET_DIR ;;
    *) xtask_target_dir=$repository_root/$CARGO_TARGET_DIR ;;
esac
cd "$repository_root"
cargo build --locked --package xtask --target-dir "$xtask_target_dir"
xtask_binary=$xtask_target_dir/debug/xtask
if [ ! -x "$xtask_binary" ]; then
    printf 'build-macos-x86_64.sh: compiled xtask was not found at %s\n' "$xtask_binary" >&2
    exit 1
fi

set -- dist build --target x86_64-apple-darwin
[ "$checks" -eq 0 ] || set -- "$@" --checks
[ "$tests" -eq 0 ] || set -- "$@" --tests
[ "$self_check" -eq 0 ] || set -- "$@" --self-check
export CARGO_MANIFEST_DIR=$repository_root/xtask
"$xtask_binary" "$@"
[ "$upload" -ne 0 ] || exit 0
if [ "$github_token_set" = x ]; then
    VIBEVM_PUBLISH_TOKEN_GITHUB=$github_token_value
    export VIBEVM_PUBLISH_TOKEN_GITHUB
fi
if [ "$legacy_token_set" = x ]; then
    VIBEVM_PUBLISH_TOKEN=$legacy_token_value
    export VIBEVM_PUBLISH_TOKEN
fi
exec "$xtask_binary" dist upload-built --target x86_64-apple-darwin
