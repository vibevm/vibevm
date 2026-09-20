FROM alpine:3.23

ARG VIBE_SHA256

RUN apk add --no-cache bash ca-certificates curl git jq openssh-client \
    && test -n "$VIBE_SHA256"
COPY vibe-bootstrap /usr/local/bin/vibe
RUN test -n "$VIBE_SHA256" \
    && echo "$VIBE_SHA256  /usr/local/bin/vibe" | sha256sum -c - \
    && chmod 0755 /usr/local/bin/vibe \
    && addgroup -S -g 10001 vibe \
    && adduser -S -D -u 10001 -G vibe -h /home/vibe vibe \
    && install -d -o vibe -g vibe /home/vibe/.vibe/opt/bin \
    && chown -R vibe:vibe /home/vibe/.vibe

ENV VIBE_SETTINGS=/home/vibe/.vibe
ENV PATH=/home/vibe/.vibe/opt/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

USER vibe
WORKDIR /workspace
CMD ["vibe", "--help"]
