import { readFile } from "node:fs/promises";
import path from "node:path";
import type { FastifyReply } from "fastify";

export async function readJsonFile(filePath: string): Promise<unknown> {
  const text = await readFile(filePath, "utf8");
  return JSON.parse(text) as unknown;
}

export function sendNoStoreJson(reply: FastifyReply, payload: unknown) {
  return reply
    .header("cache-control", "no-store")
    .type("application/json; charset=utf-8")
    .send(payload);
}

export function contentTypeFor(filePath: string): string {
  const lower = filePath.toLowerCase();
  if (lower.endsWith(".html")) return "text/html; charset=utf-8";
  if (lower.endsWith(".css")) return "text/css; charset=utf-8";
  if (lower.endsWith(".js") || lower.endsWith(".js.gz")) return "application/javascript; charset=utf-8";
  if (lower.endsWith(".wasm") || lower.endsWith(".wasm.gz")) return "application/wasm";
  if (lower.endsWith(".json") || lower.endsWith(".json.gz")) return "application/json; charset=utf-8";
  if (lower.endsWith(".png")) return "image/png";
  if (lower.endsWith(".ico")) return "image/x-icon";
  if (lower.includes("airrace_") || path.basename(lower) === "bundles") return "application/octet-stream";
  return "application/octet-stream";
}

export function contentEncodingFor(filePath: string): string | null {
  const lower = filePath.toLowerCase();
  if (lower.endsWith(".gz")) return "gzip";
  if (lower.endsWith(".br")) return "br";
  return null;
}
