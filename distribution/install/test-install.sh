#!/bin/sh

set -eu

test_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
fixture_dir=$(mktemp -d "${TMPDIR:-/tmp}/vibevm-install-test.XXXXXX")
cleanup() {
    rm -rf -- "$fixture_dir"
}
trap cleanup 0 HUP INT TERM

cat > "$fixture_dir/DISTRIBUTIONS.json" <<'EOF'
{
  "schema_version": 1,
  "product": "vibevm",
  "repository": "vibevm/vibevm",
  "version": "1.2.3",
  "tag": "v1.2.3",
  "source_commit": "0123456789012345678901234567890123456789",
  "platforms": [
    {
      "schema_version": 1,
      "product": "vibevm",
      "repository": "vibevm/vibevm",
      "version": "1.2.3",
      "tag": "v1.2.3",
      "source_commit": "0123456789012345678901234567890123456789",
      "target": "x86_64-unknown-linux-musl",
      "asset": {
        "name": "vibevm-1.2.3-x86_64-unknown-linux-musl.zip",
        "size": 10,
        "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "bootstrap": {
        "name": "vibe-bootstrap-x86_64-unknown-linux-musl",
        "size": 123,
        "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      },
      "bundle": {}
    }
  ]
}
EOF

VIBEVM_INSTALL_TEST_MODE=1 . "$test_dir/install.sh"

[ "$(vibe_install_manifest_version "$fixture_dir/DISTRIBUTIONS.json")" = '1.2.3' ]
[ "$(vibe_install_manifest_integer "$fixture_dir/DISTRIBUTIONS.json" 'schema_version')" = '1' ]
[ "$(vibe_install_manifest_string "$fixture_dir/DISTRIBUTIONS.json" 'repository')" = 'vibevm/vibevm' ]
[ "$(vibe_install_bootstrap_record "$fixture_dir/DISTRIBUTIONS.json" 'x86_64-unknown-linux-musl')" = 'vibe-bootstrap-x86_64-unknown-linux-musl|123|sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' ]
vibe_install_validate_version '1.2.3'
vibe_install_validate_version '1.2.3-rc.1+build.7'
vibe_install_validate_bootstrap \
    'vibe-bootstrap-x86_64-unknown-linux-musl' \
    '123' \
    'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' \
    'x86_64-unknown-linux-musl'

cat > "$fixture_dir/duplicate-target.json" <<'EOF'
{
  "schema_version": 1,
  "product": "vibevm",
  "repository": "vibevm/vibevm",
  "version": "1.2.3",
  "tag": "v1.2.3",
  "platforms": [
    {
      "target": "x86_64-unknown-linux-musl",
      "bootstrap": {
        "name": "vibe-bootstrap-x86_64-unknown-linux-musl",
        "size": 123,
        "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      }
    },
    {
      "target": "x86_64-unknown-linux-musl",
      "bootstrap": {
        "name": "vibe-bootstrap-x86_64-unknown-linux-musl",
        "size": 123,
        "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      }
    }
  ]
}
EOF
if vibe_install_bootstrap_record "$fixture_dir/duplicate-target.json" 'x86_64-unknown-linux-musl' >/dev/null 2>&1; then
    printf '%s\n' 'duplicate target unexpectedly accepted' >&2
    exit 1
fi
if (
    vibe_install_validate_bootstrap \
        'vibe-bootstrap-x86_64-unknown-linux-musl' \
        536870913 \
        'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' \
        'x86_64-unknown-linux-musl'
) >/dev/null 2>&1; then
    printf '%s\n' 'oversized bootstrap unexpectedly accepted' >&2
    exit 1
fi

printf '%s\n' 'install.sh offline tests passed'
