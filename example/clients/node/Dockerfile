FROM node:26-alpine
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev --no-audit --no-fund
COPY client.mjs ./
USER node
ENTRYPOINT ["node", "client.mjs"]
