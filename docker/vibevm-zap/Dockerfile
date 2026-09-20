ARG BASE_IMAGE=vibevm/vibevm:1.0.0
FROM ${BASE_IMAGE}

ARG ZAP_COMMIT

USER root
RUN apk add --no-cache socat \
    && install -d -o vibe -g vibe /home/vibe/zap-state

USER vibe
RUN test -n "$ZAP_COMMIT" \
    && git clone --branch v1.0.0 --depth 1 https://github.com/vibevm/zap.git /tmp/zap \
    && test "$(git -C /tmp/zap rev-parse HEAD)" = "$ZAP_COMMIT" \
    && report="$(vibe install -g org.vibevm.zap/zap \
      --registry /tmp/zap/vibevm/vibepacks --unattended --json)" \
    && printf '%s\n' "$report" | jq -e '.ok == true and .selectedMode == "binary"' >/dev/null \
    && rm -rf /tmp/zap

COPY --chown=vibe:vibe docker/vibevm-zap/start.sh /usr/local/bin/vibevm-zap

USER root
RUN chmod 0755 /usr/local/bin/vibevm-zap

USER vibe
ENV ZAP_STATE_DIR=/home/vibe/zap-state
EXPOSE 8080
CMD ["vibevm-zap"]
