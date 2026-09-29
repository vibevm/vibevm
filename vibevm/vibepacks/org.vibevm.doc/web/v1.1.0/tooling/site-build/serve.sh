#!/usr/bin/env bash
set -euo pipefail

: "${VIBE_PROJECT_ROOT:?vibe run did not supply VIBE_PROJECT_ROOT}"
: "${VIBE_EXECUTABLE:?vibe run did not supply VIBE_EXECUTABLE}"

skip_build=false
for argument in "$@"; do
  if [[ "$argument" == "--no-build" ]]; then
    skip_build=true
  fi
done

if [[ "$skip_build" == false ]]; then
  package_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
  node "$package_root/tooling/site-build/build.mjs"
fi

package_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
exec node "$package_root/tooling/local-site/run.mjs" --no-build --no-install "$@"
