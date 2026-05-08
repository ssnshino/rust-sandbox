import { createReadStream } from "node:fs";
import { readFile } from "node:fs/promises";
import path from "node:path";
import type { FastifyInstance, FastifyReply } from "fastify";
import { config, safeJoin } from "../lib/paths.js";
import { contentEncodingFor, contentTypeFor } from "../lib/files.js";

export async function registerHomeRoutes(app: FastifyInstance) {
  app.get("/", async (_request, reply) => sendPublicFile(reply, "index.html"));
  app.get("/favicon.ico", async (_request, reply) => sendPublicFile(reply, "airrace3d/TemplateData/favicon.ico"));
}

async function sendPublicFile(reply: FastifyReply, requestPath: string) {
  const fullPath = safeJoin(config.publicRoot, requestPath);
  if (!fullPath) return reply.code(400).send({ error: "invalid path" });

  try {
    await readFile(fullPath);
  } catch {
    return reply.code(404).send({ error: "not found" });
  }

  const encoding = contentEncodingFor(fullPath);
  if (encoding) reply.header("content-encoding", encoding);
  return reply
    .header("cache-control", "public, max-age=300")
    .type(contentTypeFor(path.basename(fullPath)))
    .send(createReadStream(fullPath));
}
