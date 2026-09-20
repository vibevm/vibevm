ARG BASE_IMAGE=vibevm/vibevm:1.0.0
FROM ${BASE_IMAGE}

ARG NPM_CONFIG_REGISTRY=https://registry.npmjs.org

USER root
RUN apk add --no-cache \
      build-base clang cmake curl libgit2-dev nodejs npm openssl-dev pkgconf python3 \
    && curl --proto '=https' --tlsv1.2 --fail --silent --show-error \
      https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.93.0 \
    && npm config set registry "$NPM_CONFIG_REGISTRY" \
    && npm install --global pnpm@10.33.2 typescript@5.9.3 \
    && npm cache clean --force

ENV RUSTUP_HOME=/root/.rustup
ENV CARGO_HOME=/root/.cargo
ENV PATH=/root/.cargo/bin:/home/vibe/.vibe/opt/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ENV NPM_CONFIG_REGISTRY=${NPM_CONFIG_REGISTRY}

WORKDIR /workspace
CMD ["sh"]
