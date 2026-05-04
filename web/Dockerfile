FROM node:22-bookworm-slim

WORKDIR /workspace/web

COPY web/package*.json ./
RUN npm install

COPY web/ ./
RUN npm run build

EXPOSE 3000

CMD ["npm", "run", "dev"]
