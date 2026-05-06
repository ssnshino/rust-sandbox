import { createReadStream } from "node:fs";
import { readFile } from "node:fs/promises";
import path from "node:path";
import type { FastifyInstance, FastifyReply } from "fastify";
import { config, safeJoin } from "../lib/paths.js";
import { contentEncodingFor, contentTypeFor, readJsonFile, sendNoStoreJson } from "../lib/files.js";

const aircraftCatalog = {
  version: "airrace3d-aircraft-v3",
  defaultAircraftId: "skylancer",
  aircrafts: [
    { id: "skylancer", name: "スカイランサー号", nameEn: "Skylancer", summary: "平均的な優等生。初見コースの基準機。", summaryEn: "Balanced all-rounder. Best baseline for new courses.", speedMul: 1.0, boostMul: 1.0, turnMul: 1.0, climbMul: 1.0, color: "rgba(125, 211, 252, 0.78)", stroke: "#e0f2fe", shape: "standard" },
    { id: "thunderbolt", name: "サンダーボルト号", nameEn: "Thunderbolt", summary: "きびきび旋回。テクニカル向け。", summaryEn: "Sharp turning. Great for technical sections.", speedMul: 0.94, boostMul: 0.94, turnMul: 1.24, climbMul: 1.08, color: "rgba(250, 204, 21, 0.78)", stroke: "#fef3c7", shape: "wide" },
    { id: "shootingstar", name: "シューティングスター号", nameEn: "Shooting Star", summary: "高速番長。直線と大カーブで強い。", summaryEn: "Top speed specialist. Dominates straights and long bends.", speedMul: 1.08, boostMul: 1.16, turnMul: 0.82, climbMul: 0.94, color: "rgba(248, 113, 113, 0.80)", stroke: "#fee2e2", shape: "dart" },
    { id: "spiralfang", name: "スパイラルファング号", nameEn: "Spiral Fang", summary: "ピーキーな軽量機。反応最速。", summaryEn: "Twitchy lightweight. Fastest response.", speedMul: 0.98, boostMul: 0.98, turnMul: 1.32, climbMul: 1.14, color: "rgba(192, 132, 252, 0.82)", stroke: "#f3e8ff", shape: "fang" },
    { id: "ironhawk", name: "アイアンホーク号", nameEn: "Iron Hawk", summary: "重いけど安定。崩れにくい。", summaryEn: "Heavy but stable. Hard to destabilize.", speedMul: 1.03, boostMul: 1.08, turnMul: 0.9, climbMul: 0.92, color: "rgba(74, 222, 128, 0.76)", stroke: "#dcfce7", shape: "heavy" },
  ],
};

export async function registerAirRace3dRoutes(app: FastifyInstance) {
  app.get("/api/airrace3d/round-index", async (_request, reply) => {
    const rounds = await loadRounds();
    const ids = Object.keys(rounds)
      .map((key) => Number(key))
      .filter((round) => Number.isInteger(round) && round > 0)
      .sort((a, b) => a - b);
    return sendNoStoreJson(reply, { finalRound: ids.at(-1) ?? 1, rounds: ids });
  });

  app.get<{ Params: { round: string } }>("/api/airrace3d/course/:round", async (request, reply) => {
    const rounds = await loadRounds();
    const course = rounds[request.params.round];
    if (!course) return reply.code(404).send({ error: "round not found" });
    return sendNoStoreJson(reply, course);
  });

  app.get("/api/airrace3d/field-catalog", async (_request, reply) => {
    const catalogPath = path.join(config.airrace3dRoot, "StreamingAssets/AirRace/airrace3d_field.json");
    const fallback = { version: "airrace3d-field-v1", globalField: { minX: -32000, maxX: 32000, minZ: -32000, maxZ: 32000, margin: 640 }, overrides: [] };
    return sendNoStoreJson(reply, await readJsonOrFallback(catalogPath, fallback));
  });

  app.get("/api/airrace3d/world-object-catalog", async (_request, reply) => {
    const catalogPath = path.join(config.airrace3dRoot, "StreamingAssets/AirRace/world_object_catalog.json");
    return sendNoStoreJson(reply, await readJsonOrFallback(catalogPath, { version: "airrace3d-world-v1", objects: [] }));
  });

  app.get("/api/airrace3d/round-world-catalog", async (_request, reply) => {
    return sendCatalog(reply, "round_world_catalog.json");
  });

  app.get("/api/airrace3d/aircraft-catalog", async (_request, reply) => {
    const catalogPath = path.join(config.airrace3dRoot, "StreamingAssets/AirRace/aircraft_catalog.json");
    return sendNoStoreJson(reply, await readJsonOrFallback(catalogPath, aircraftCatalog));
  });

  app.get("/api/airrace3d/aircraft-prefab-catalog", async (_request, reply) => {
    return sendCatalog(reply, "aircraft_prefab_catalog.json");
  });

  app.get("/airrace3d", async (_request, reply) => sendAirRaceFile(reply, "index.html"));
  app.get("/airrace3d/", async (_request, reply) => sendAirRaceFile(reply, "index.html"));
  app.get<{ Params: { "*": string } }>("/airrace3d/*", async (request, reply) => {
    const requestPath = request.params["*"] || "index.html";
    return sendAirRaceFile(reply, requestPath);
  });
}

async function sendCatalog(reply: FastifyReply, filename: string) {
  const catalogPath = path.join(config.airrace3dRoot, "StreamingAssets/AirRace", filename);
  try {
    return sendNoStoreJson(reply, await readJsonFile(catalogPath));
  } catch {
    return reply.code(404).send({ error: `${filename} not found` });
  }
}

async function sendAirRaceFile(reply: FastifyReply, requestPath: string) {
  const fullPath = safeJoin(config.airrace3dRoot, requestPath);
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
    .type(contentTypeFor(fullPath))
    .send(createReadStream(fullPath));
}

async function loadRounds(): Promise<Record<string, unknown>> {
  const publicRoundsPath = path.join(config.airrace3dRoot, "StreamingAssets/AirRace/airrace_rounds.json");
  try {
    return (await readJsonFile(publicRoundsPath)) as Record<string, unknown>;
  } catch {
    return (await readJsonFile(path.join(config.airraceDataRoot, "airrace_rounds.json"))) as Record<string, unknown>;
  }
}

async function readJsonOrFallback(filePath: string, fallback: unknown): Promise<unknown> {
  try {
    return await readJsonFile(filePath);
  } catch {
    return fallback;
  }
}
