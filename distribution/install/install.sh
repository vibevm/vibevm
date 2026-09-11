#!/bin/sh

set -eu

VIBEVM_RELEASE_REPOSITORY="vibevm/vibevm"
VIBEVM_RELEASE_ORIGIN="https://github.com/${VIBEVM_RELEASE_REPOSITORY}/releases"
VIBEVM_DISTRIBUTIONS_ASSET="DISTRIBUTIONS.json"
VIBEVM_MANIFEST_MAX_BYTES=4194304
VIBEVM_BOOTSTRAP_MAX_BYTES=536870912

vibe_install_say() {
    printf '%s\n' "$*"
}

vibe_install_fail() {
    printf 'vibevm installer: %s\n' "$*" >&2
    exit 1
}

vibe_install_usage() {
    cat <<'EOF'
Install the latest VibeVM release:
  curl -fsSL https://vibevm.org/install.sh | bash

Install an explicit release or force a fresh local generation:
  curl -fsSL https://vibevm.org/install.sh | bash -s -- --version 1.0.0
  curl -fsSL https://vibevm.org/install.sh | bash -s -- --force

Options:
  --version VERSION  Install an explicit SemVer release.
  --force            Reinstall even when the downloaded bytes are unchanged.
  -h, --help         Show this help.
EOF
}

vibe_install_validate_version() {
    printf '%s\n' "$1" | grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
}

vibe_install_detect_target() {
    vibe_install_os=$(uname -s 2>/dev/null || true)
    vibe_install_arch=$(uname -m 2>/dev/null || true)

    case "$vibe_install_os" in
        Linux)
            case "$vibe_install_arch" in
                x86_64|amd64)
                    printf '%s\n' 'x86_64-unknown-linux-musl'
                    ;;
                *)
                    vibe_install_fail "unsupported Linux architecture '$vibe_install_arch'; this release supports x86_64 only"
                    ;;
            esac
            ;;
        Darwin)
            case "$vibe_install_arch" in
                arm64|aarch64)
                    printf '%s\n' 'aarch64-apple-darwin'
                    ;;
                x86_64|amd64)
                    vibe_install_translated=$(sysctl -in sysctl.proc_translated 2>/dev/null || true)
                    vibe_install_has_arm=$(sysctl -in hw.optional.arm64 2>/dev/null || true)
                    if [ "$vibe_install_translated" = '1' ] || [ "$vibe_install_has_arm" = '1' ]; then
                        printf '%s\n' 'aarch64-apple-darwin'
                    else
                        printf '%s\n' 'x86_64-apple-darwin'
                    fi
                    ;;
                *)
                    vibe_install_fail "unsupported macOS architecture '$vibe_install_arch'; this release supports x86_64 and arm64"
                    ;;
            esac
            ;;
        *)
            vibe_install_fail "unsupported operating system '$vibe_install_os'; use install.ps1 on native Windows"
            ;;
    esac
}

vibe_install_download() {
    vibe_install_download_url=$1
    vibe_install_download_destination=$2
    vibe_install_download_maximum=$3

    if ! command -v curl >/dev/null 2>&1; then
        vibe_install_fail 'curl is required to download VibeVM'
    fi
    curl -fsSL --retry 3 --connect-timeout 20 --max-time 1800 \
        --speed-limit 1024 --speed-time 60 \
        --proto '=https' --proto-redir '=https' \
        --max-filesize "$vibe_install_download_maximum" \
        -H 'Cache-Control: no-cache' \
        "$vibe_install_download_url" -o "$vibe_install_download_destination" ||
        vibe_install_fail "download failed or exceeded $vibe_install_download_maximum bytes: $vibe_install_download_url"
    vibe_install_download_size=$(wc -c < "$vibe_install_download_destination" | tr -d '[:space:]')
    case "$vibe_install_download_size" in
        ''|*[!0-9]*) vibe_install_fail 'download size could not be measured safely' ;;
    esac
    if [ "$vibe_install_download_size" -gt "$vibe_install_download_maximum" ]; then
        vibe_install_fail "download exceeded the $vibe_install_download_maximum-byte policy limit"
    fi
}

vibe_install_manifest_string() {
    awk -v wanted="$2" '
        /^  "platforms"[[:space:]]*:/ { exit }
        /^  "[A-Za-z_]+"[[:space:]]*:/ {
            line = $0
            key = line
            sub(/^[[:space:]]*"/, "", key)
            sub(/".*/, "", key)
            if (key != wanted) {
                next
            }
            sub(/^[^:]*:[[:space:]]*"/, "", line)
            sub(/"[[:space:]]*,?[[:space:]]*$/, "", line)
            print line
            exit
        }
    ' "$1"
}

vibe_install_manifest_integer() {
    awk -v wanted="$2" '
        /^  "platforms"[[:space:]]*:/ { exit }
        /^  "[A-Za-z_]+"[[:space:]]*:/ {
            line = $0
            key = line
            sub(/^  "/, "", key)
            sub(/".*/, "", key)
            if (key != wanted) {
                next
            }
            sub(/^[^:]*:[[:space:]]*/, "", line)
            sub(/,[[:space:]]*$/, "", line)
            gsub(/[[:space:]]/, "", line)
            print line
            exit
        }
    ' "$1"
}

vibe_install_manifest_version() {
    vibe_install_manifest_string "$1" 'version'
}

vibe_install_bootstrap_record() {
    awk -v wanted="$2" '
        function string_value(line) {
            sub(/^[^:]*:[[:space:]]*"/, "", line)
            sub(/"[[:space:]]*,?[[:space:]]*$/, "", line)
            return line
        }
        function integer_value(line) {
            sub(/^[^:]*:[[:space:]]*/, "", line)
            sub(/,[[:space:]]*$/, "", line)
            gsub(/[[:space:]]/, "", line)
            return line
        }
        /^      "target"[[:space:]]*:/ {
            current = (string_value($0) == wanted)
            if (current) {
                target_count++
            }
            bootstrap = 0
            next
        }
        current && /^      "bootstrap"[[:space:]]*:[[:space:]]*\{/ {
            bootstrap = 1
            next
        }
        current && bootstrap && /^        "name"[[:space:]]*:/ {
            name = string_value($0)
            next
        }
        current && bootstrap && /^        "size"[[:space:]]*:/ {
            size = integer_value($0)
            next
        }
        current && bootstrap && /^        "digest"[[:space:]]*:/ {
            digest = string_value($0)
            record = name "|" size "|" digest
            record_count++
            bootstrap = 0
        }
        END {
            if (target_count == 1 && record_count == 1) {
                print record
            } else {
                exit 1
            }
        }
    ' "$1"
}

vibe_install_sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{ print $1 }'
        return
    fi
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{ print $1 }'
        return
    fi
    vibe_install_fail 'sha256sum or shasum is required to verify the VibeVM bootstrap'
}

vibe_install_validate_bootstrap() {
    vibe_install_bootstrap_name=$1
    vibe_install_bootstrap_size=$2
    vibe_install_bootstrap_digest=$3
    vibe_install_bootstrap_target=$4

    case "$vibe_install_bootstrap_name" in
        ''|*[!A-Za-z0-9._-]*|.|..)
            vibe_install_fail 'release manifest contains an unsafe bootstrap filename'
            ;;
    esac
    if [ "$vibe_install_bootstrap_name" != "vibe-bootstrap-$vibe_install_bootstrap_target" ]; then
        vibe_install_fail "release manifest names an unexpected bootstrap for $vibe_install_bootstrap_target"
    fi
    case "$vibe_install_bootstrap_size" in
        ''|0|*[!0-9]*)
            vibe_install_fail 'release manifest contains an invalid bootstrap size'
            ;;
    esac
    if [ "$vibe_install_bootstrap_size" -gt "$VIBEVM_BOOTSTRAP_MAX_BYTES" ]; then
        vibe_install_fail "release bootstrap exceeds the $VIBEVM_BOOTSTRAP_MAX_BYTES-byte policy limit"
    fi
    if ! printf '%s\n' "$vibe_install_bootstrap_digest" | grep -Eq '^sha256:[0-9a-f]{64}$'; then
        vibe_install_fail 'release manifest contains an invalid bootstrap SHA-256 digest'
    fi
}

vibe_install_cleanup() {
    if [ -n "${vibe_install_temp_dir:-}" ] && [ -d "$vibe_install_temp_dir" ]; then
        rm -rf -- "$vibe_install_temp_dir"
    fi
}

vibe_install_main() {
    vibe_install_requested_version=''
    vibe_install_force=0

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --version)
                [ "$#" -ge 2 ] || vibe_install_fail '--version requires a value'
                vibe_install_requested_version=$2
                shift 2
                ;;
            --force)
                vibe_install_force=1
                shift
                ;;
            -h|--help)
                vibe_install_usage
                return 0
                ;;
            *)
                vibe_install_fail "unknown argument '$1'; use --help for supported options"
                ;;
        esac
    done

    if [ -n "$vibe_install_requested_version" ] && ! vibe_install_validate_version "$vibe_install_requested_version"; then
        vibe_install_fail "invalid SemVer '$vibe_install_requested_version'"
    fi

    if [ "$(id -u 2>/dev/null || printf '%s' 1)" = '0' ] &&
       { [ -n "${SUDO_USER:-}" ] || [ -n "${SUDO_UID:-}" ]; }; then
        vibe_install_fail 'do not run this installer with sudo; VibeVM installs into the current user profile'
    fi

    vibe_install_target=$(vibe_install_detect_target)
    vibe_install_temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/vibevm-install.XXXXXX") ||
        vibe_install_fail 'could not create a temporary directory'
    trap 'vibe_install_cleanup' 0
    trap 'exit 130' HUP INT TERM

    vibe_install_manifest_path="$vibe_install_temp_dir/$VIBEVM_DISTRIBUTIONS_ASSET"
    vibe_install_nonce="$(date +%s 2>/dev/null || printf '0')-$$"
    if [ -n "$vibe_install_requested_version" ]; then
        vibe_install_manifest_url="$VIBEVM_RELEASE_ORIGIN/download/v$vibe_install_requested_version/$VIBEVM_DISTRIBUTIONS_ASSET"
    else
        vibe_install_manifest_url="$VIBEVM_RELEASE_ORIGIN/latest/download/$VIBEVM_DISTRIBUTIONS_ASSET"
    fi

    vibe_install_say "Fetching VibeVM release metadata for $vibe_install_target..."
    vibe_install_download \
        "$vibe_install_manifest_url?vvm_refresh=$vibe_install_nonce" \
        "$vibe_install_manifest_path" \
        "$VIBEVM_MANIFEST_MAX_BYTES"

    vibe_install_schema=$(vibe_install_manifest_integer "$vibe_install_manifest_path" 'schema_version')
    if [ "$vibe_install_schema" != '1' ]; then
        vibe_install_fail "release manifest schema '$vibe_install_schema' is unsupported"
    fi
    vibe_install_version=$(vibe_install_manifest_version "$vibe_install_manifest_path")
    if [ -z "$vibe_install_version" ] || ! vibe_install_validate_version "$vibe_install_version"; then
        vibe_install_fail 'release manifest does not contain a valid version'
    fi
    vibe_install_product=$(vibe_install_manifest_string "$vibe_install_manifest_path" 'product')
    vibe_install_repository=$(vibe_install_manifest_string "$vibe_install_manifest_path" 'repository')
    vibe_install_tag=$(vibe_install_manifest_string "$vibe_install_manifest_path" 'tag')
    if [ "$vibe_install_product" != 'vibevm' ] ||
       [ "$vibe_install_repository" != "$VIBEVM_RELEASE_REPOSITORY" ] ||
       [ "$vibe_install_tag" != "v$vibe_install_version" ]; then
        vibe_install_fail 'release manifest identity is invalid'
    fi
    if [ -n "$vibe_install_requested_version" ] && [ "$vibe_install_version" != "$vibe_install_requested_version" ]; then
        vibe_install_fail "release manifest version '$vibe_install_version' does not match requested '$vibe_install_requested_version'"
    fi

    vibe_install_record=$(vibe_install_bootstrap_record "$vibe_install_manifest_path" "$vibe_install_target" || true)
    [ -n "$vibe_install_record" ] ||
        vibe_install_fail "release manifest has no bootstrap for $vibe_install_target"
    vibe_install_bootstrap_name=${vibe_install_record%%|*}
    vibe_install_record_rest=${vibe_install_record#*|}
    vibe_install_bootstrap_size=${vibe_install_record_rest%%|*}
    vibe_install_bootstrap_digest=${vibe_install_record_rest#*|}
    vibe_install_validate_bootstrap \
        "$vibe_install_bootstrap_name" \
        "$vibe_install_bootstrap_size" \
        "$vibe_install_bootstrap_digest" \
        "$vibe_install_target"

    vibe_install_release_base="$VIBEVM_RELEASE_ORIGIN/download/v$vibe_install_version"
    vibe_install_bootstrap_path="$vibe_install_temp_dir/$vibe_install_bootstrap_name"
    vibe_install_say "Downloading VibeVM $vibe_install_version bootstrap..."
    vibe_install_download \
        "$vibe_install_release_base/$vibe_install_bootstrap_name?vvm_refresh=$vibe_install_nonce" \
        "$vibe_install_bootstrap_path" \
        "$vibe_install_bootstrap_size"

    vibe_install_actual_size=$(wc -c < "$vibe_install_bootstrap_path" | tr -d '[:space:]')
    if [ "$vibe_install_actual_size" != "$vibe_install_bootstrap_size" ]; then
        vibe_install_fail "bootstrap size mismatch (expected $vibe_install_bootstrap_size bytes, received $vibe_install_actual_size)"
    fi
    vibe_install_expected_sha=${vibe_install_bootstrap_digest#sha256:}
    vibe_install_actual_sha=$(vibe_install_sha256 "$vibe_install_bootstrap_path")
    if [ "$vibe_install_actual_sha" != "$vibe_install_expected_sha" ]; then
        vibe_install_fail 'bootstrap SHA-256 mismatch; the downloaded file was not executed'
    fi

    chmod 700 "$vibe_install_bootstrap_path" ||
        vibe_install_fail 'could not mark the verified bootstrap executable'

    vibe_install_say 'Installing vibe, vibe-index, and the matching VibeVM sources...'
    set -- self bootstrap \
        --manifest "$vibe_install_manifest_path" \
        --version "$vibe_install_version" \
        --release-base "$vibe_install_release_base"
    if [ "$vibe_install_force" -eq 1 ]; then
        set -- "$@" --force
    fi
    "$vibe_install_bootstrap_path" "$@" || {
        vibe_install_status=$?
        vibe_install_fail "verified bootstrap exited with status $vibe_install_status"
    }

    vibe_install_say 'VibeVM is installed; the active selector is shown above.'
    vibe_install_say 'Follow the PATH guidance above, then run `vibe self current` to inspect it.'
}

if [ "${VIBEVM_INSTALL_TEST_MODE:-0}" != '1' ]; then
    vibe_install_main "$@"
fi
