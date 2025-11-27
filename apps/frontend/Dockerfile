# syntax=docker/dockerfile:1.6

FROM node:20-bookworm AS builder

WORKDIR /app

ARG BACKEND_BASE_URL
ENV BACKEND_BASE_URL=$BACKEND_BASE_URL

ENV PNPM_HOME="/pnpm" \
    PATH="/pnpm:$PATH" \
    NEXT_TELEMETRY_DISABLED=1

RUN corepack enable

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps apps
COPY packages packages

RUN pnpm install --frozen-lockfile
RUN pnpm --filter @nommie/frontend build

FROM node:20-bookworm-slim AS runner

WORKDIR /app

ENV NODE_ENV=production \
    PORT=3000 \
    NEXT_TELEMETRY_DISABLED=1

# Copy the standalone build output
COPY --from=builder /app/apps/frontend/.next/standalone ./
# Copy static files to where the server expects them (relative to apps/frontend/)
COPY --from=builder /app/apps/frontend/.next/static ./apps/frontend/.next/static
COPY --from=builder /app/apps/frontend/public ./apps/frontend/public

EXPOSE 3000

CMD ["node", "apps/frontend/server.js"]


