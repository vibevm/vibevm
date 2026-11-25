#!/bin/sh
# source_build recipe for Linux and macOS
# (spec://org.vibevm.core/vibevm/common/PROP-059#source-build): copy the
# package tree from $VIBE_PACKAGE into $VIBE_OUT/site without build output,
# then write bin/vibevm-doc. $VIBE_OUT is a staging path, so the wrapper
# finds the site relative to itself and nothing bakes an absolute path in.
set -eu
mkdir -p "$VIBE_OUT/site" "$VIBE_OUT/bin"
(cd "$VIBE_PACKAGE" && tar cf - \
  --exclude=.vite --exclude=dist --exclude=dist-embedded \
  --exclude=node_modules --exclude=playwright-report --exclude=server \
  --exclude=server-embedded --exclude=target --exclude=test-results \
  --exclude=tmp .) | (cd "$VIBE_OUT/site" && tar xf -)
cat > "$VIBE_OUT/bin/vibevm-doc" <<'WRAPPER'
#!/bin/sh
here=$(cd "$(dirname "$0")" && pwd)
exec node "$here/../site/tooling/local-site/run.mjs" "$@"
WRAPPER
chmod 755 "$VIBE_OUT/bin/vibevm-doc"
