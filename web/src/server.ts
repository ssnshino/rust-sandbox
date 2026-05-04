import Fastify from "fastify";
import { config } from "./lib/paths.js";
import { registerAirRace3dRoutes } from "./routes/airrace3d.js";

const app = Fastify({ logger: true });

app.get("/healthz", async () => ({
  ok: true,
  service: "unity-rust-games-web",
  airrace3dRoot: config.airrace3dRoot,
  airraceDataRoot: config.airraceDataRoot,
}));

await registerAirRace3dRoutes(app);

try {
  await app.listen({ host: config.host, port: config.port });
} catch (error) {
  app.log.error(error);
  process.exit(1);
}
