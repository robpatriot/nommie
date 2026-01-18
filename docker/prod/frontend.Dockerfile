# syntax=docker/dockerfile:1.6

FROM node:20-bookworm AS builder

WORKDIR /app

ENV PNPM_HOME="/pnpm" \
    PATH="/pnpm:$PATH" \
    NEXT_TELEMETRY_DISABLED=1

RUN corepack enable

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps apps
COPY packages packages

# Copy the build-time env file into the image
COPY docker/prod/backend_base_url.env /tmp/backend_base_url.env

RUN pnpm install --frozen-lockfile

RUN set -a && . /tmp/backend_base_url.env && set +a && \
    if [ -z "${BACKEND_BASE_URL:-}" ]; then \
      echo >&2 "❌ FRONTEND BUILD CONFIG ERROR"; \
      echo >&2 "   Missing BACKEND_BASE_URL Expected in: docker/prod/backend_base_url.env"; \
      exit 2; \
    fi && \
    pnpm --filter @nommie/frontend exec next build

    FROM node:20-bookworm-slim AS runner

WORKDIR /app

ENV NODE_ENV=production \
    PORT=3000 \
    NEXT_TELEMETRY_DISABLED=1

# Ensure runtime does not run as root
RUN chown -R node:node /app
USER node

# Copy the standalone build output
COPY --from=builder /app/apps/frontend/.next/standalone ./
# Copy static files to where the server expects them (relative to apps/frontend/)
COPY --from=builder /app/apps/frontend/.next/static ./apps/frontend/.next/static
COPY --from=builder /app/apps/frontend/public ./apps/frontend/public

EXPOSE 3000

CMD ["node", "apps/frontend/server.js"]


