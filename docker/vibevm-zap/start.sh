#!/bin/sh
set -eu

zap-server --state-dir "${ZAP_STATE_DIR:-/home/vibe/zap-state}" "$@" &
zap_pid=$!
trap 'kill "$zap_pid" 2>/dev/null || true' INT TERM EXIT

attempt=0
while [ "$attempt" -lt 60 ]; do
    if ! kill -0 "$zap_pid" 2>/dev/null; then
        wait "$zap_pid"
        exit $?
    fi
    if nc -z 127.0.0.1 4174 2>/dev/null; then
        exec socat TCP-LISTEN:8080,fork,reuseaddr,bind=0.0.0.0 TCP:127.0.0.1:4174
    fi
    attempt=$((attempt + 1))
    sleep 1
done

printf '%s\n' 'vibevm-zap: Zap web interface did not become ready on 127.0.0.1:4174' >&2
exit 1
