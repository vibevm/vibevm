#!/bin/sh
# Take the Content-Security-Policy of the render this container was
# handed, before nginx reads its configuration.
#
# nginx's own entrypoint runs everything in `/docker-entrypoint.d` and
# then starts the server, so this is the one moment at which the
# directory is mounted and the configuration has not been parsed yet.
#
# Why it is copied rather than written here: the policy names one hash
# per inline script, and the hashes are computed by the build from the
# bytes of the pages it wrote (X-035, X-044). The build therefore writes
# the fragment, beside its own state, inside the directory it produced —
# so the policy and the pages it describes are always the same render,
# and a container restarted against an older directory serves that
# directory's policy rather than a newer one it was built with.
#
# A missing fragment is not an error. The image ships an empty policy
# that defines the variable, so nginx starts either way: a domain that
# refused to come up because a render had not happened yet would be a
# worse failure than one restart without a header.

set -e

render=/usr/share/nginx/html/.vibe-site/csp.conf
installed=/etc/nginx/csp.conf

if [ -f "$render" ]; then
    cp "$render" "$installed"
    echo "csp: policy taken from the render ($(wc -c <"$installed" | tr -d ' ') bytes)"
else
    echo "csp: the directory carries no policy; serving without one"
fi
