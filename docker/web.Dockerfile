# syntax=docker/dockerfile:1

FROM node:22-bookworm-slim AS build
WORKDIR /app

RUN corepack enable && corepack prepare pnpm@10.17.1 --activate

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/web/package.json apps/web/package.json
RUN pnpm install --frozen-lockfile

COPY apps/web apps/web
RUN pnpm --filter @docufill/web build \
    && pnpm --filter @docufill/web deploy --prod --legacy /out \
    && cp -R apps/web/build /out/build

FROM node:22-bookworm-slim
WORKDIR /app

RUN useradd --system --uid 10001 --home /app --shell /usr/sbin/nologin docufill

COPY --from=build --chown=docufill:docufill /out/package.json ./
COPY --from=build --chown=docufill:docufill /out/node_modules ./node_modules
COPY --from=build --chown=docufill:docufill /out/build ./build

ENV NODE_ENV=production \
    HOST=0.0.0.0 \
    PORT=3000 \
    PROTOCOL_HEADER=x-forwarded-proto \
    HOST_HEADER=host

USER docufill
EXPOSE 3000
CMD ["node", "build"]
