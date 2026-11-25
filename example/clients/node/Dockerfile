FROM node:26-alpine
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev --no-audit --no-fund
COPY client.mjs ./
USER node
# One-shot client: it publishes or receives COUNT messages and exits, so there's nothing to probe.
HEALTHCHECK NONE
ENTRYPOINT ["node", "client.mjs"]
