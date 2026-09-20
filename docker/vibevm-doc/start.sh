#!/bin/sh
set -eu
/usr/local/bin/csp-from-render
exec nginx -g 'daemon off;'
