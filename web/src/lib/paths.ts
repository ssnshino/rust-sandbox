import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(here, "../..");
const repoRoot = path.resolve(webRoot, "..");

export const config = {
  port: Number(process.env.PORT ?? "3000"),
  host: process.env.HOST ?? "0.0.0.0",
  airrace3dRoot: path.resolve(process.env.AIRRACE3D_ROOT ?? path.join(webRoot, "public/airrace3d")),
  airraceDataRoot: path.resolve(process.env.AIRRACE_DATA_ROOT ?? path.join(repoRoot, "app/src/airrace")),
};

export function safeJoin(root: string, requestPath: string): string | null {
  const decoded = decodeURIComponent(requestPath).replace(/^\/+/, "");
  const parts = decoded.split(/[\\/]+/).filter(Boolean);
  if (parts.length === 0) return null;
  if (parts.some((part) => part === "." || part === "..")) return null;

  const fullPath = path.resolve(root, ...parts);
  const relative = path.relative(root, fullPath);
  if (relative.startsWith("..") || path.isAbsolute(relative)) return null;
  return fullPath;
}
