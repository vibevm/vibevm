ARG DEV_IMAGE=vibevm/vibevm-dev:1.0.10
ARG BASE_IMAGE=vibevm/vibevm:1.0.10

FROM ${DEV_IMAGE} AS build
ARG NPM_CONFIG_REGISTRY=https://registry.npmjs.org
ENV NPM_CONFIG_REGISTRY=${NPM_CONFIG_REGISTRY}

COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/package.json \
     vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/pnpm-lock.yaml \
     vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/pnpm-workspace.yaml /web/
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/design/package.json /web/design/
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/package.json /web/site/
RUN cd /web && pnpm install --frozen-lockfile

COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/ /web/
COPY . /checkout
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/docker/site.toml /site.toml
# The live analytics property comes from the deployment, never the repository
# (##SITE-ANALYTICS); empty keeps the committed default, which renders no tag.
ARG UMAMI_WEBSITE_ID=""
RUN if [ -n "$UMAMI_WEBSITE_ID" ]; then \
        printf '%s\n' "$UMAMI_WEBSITE_ID" \
            | grep -Eqx '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' \
            || { echo "UMAMI_WEBSITE_ID is not a UUID" >&2; exit 1; }; \
        sed -i "s/^website_id = \"\"\$/website_id = \"${UMAMI_WEBSITE_ID}\"/" /site.toml; \
        grep -qx "website_id = \"${UMAMI_WEBSITE_ID}\"" /site.toml \
            || { echo "site.toml has no empty website_id to fill" >&2; exit 1; }; \
    fi \
    && vibe doc build-site --config /site.toml --out /out --web /web

FROM ${BASE_IMAGE}
USER root
RUN apk add --no-cache nginx nodejs \
    && rm -f /etc/nginx/http.d/default.conf
COPY --from=build /out/ /usr/share/nginx/html/
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/docker/nginx.conf /etc/nginx/http.d/vibevm-site.conf
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/docker/csp.default.conf /etc/nginx/csp.conf
COPY vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/docker/csp-from-render.sh /usr/local/bin/csp-from-render
COPY docker/vibevm-doc/start.sh /usr/local/bin/vibevm-doc
RUN chmod 0755 /usr/local/bin/csp-from-render /usr/local/bin/vibevm-doc \
    && /usr/local/bin/csp-from-render \
    && nginx -t
EXPOSE 80
CMD ["vibevm-doc"]
