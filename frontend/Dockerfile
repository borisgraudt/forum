# syntax=docker/dockerfile:1

FROM node:22-bookworm-slim AS builder
WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .

ARG PUBLIC_API_URL=http://localhost:3000/api/v1
ENV PUBLIC_API_URL=${PUBLIC_API_URL}

RUN npm run build

FROM node:22-bookworm-slim AS runtime
WORKDIR /app

ENV HOST=0.0.0.0 \
    PORT=4321 \
    NODE_ENV=production

COPY --from=builder /app/dist ./dist
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/node_modules ./node_modules

USER node
EXPOSE 4321

CMD ["node", "./dist/server/entry.mjs"]
