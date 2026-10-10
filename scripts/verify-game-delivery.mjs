import fs from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { createGunzip, createBrotliDecompress } from 'node:zlib';
import { pathToFileURL } from 'node:url';

export async function hashFile(file, decoded = false) {
  let stream = createReadStream(file);
  if (decoded && file.endsWith('.gz')) stream = stream.pipe(createGunzip());
  if (decoded && file.endsWith('.br')) stream = stream.pipe(createBrotliDecompress());
  const hash = createHash('sha256');
  for await (const chunk of stream) hash.update(chunk);
  return hash.digest('hex');
}
export function safeRelative(value) {
  if (typeof value !== 'string' || !value || /[\\?#:%]/.test(value) || path.posix.isAbsolute(value) || value.split('/').some(p => !p || p === '.' || p === '..')) throw new Error(`Unsafe artifact reference: ${value}`);
  return value;
}
async function inventory(root, prefix = '') {
  const files = [];
  for (const item of await fs.readdir(path.join(root, prefix), { withFileTypes: true })) {
    const rel = prefix ? `${prefix}/${item.name}` : item.name;
    safeRelative(rel);
    if (item.isSymbolicLink()) throw new Error(`Symlink forbidden: ${rel}`);
    if (item.isDirectory()) files.push(...await inventory(root, rel));
    else if (item.isFile()) files.push(rel);
    else throw new Error(`Unsupported file: ${rel}`);
  }
  return files.sort();
}
export async function verifyGame(root, game) {
  if ((await fs.lstat(root)).isSymbolicLink()) throw new Error('Game root symlink forbidden');
  const files = await inventory(root);
  const available = new Set(files);
  const required = new Set(['index.html']);
  const html = await fs.readFile(path.join(root, 'index.html'), 'utf8');
  if (!/\b(?:var|let|const)\s+buildUrl\s*=\s*["']Build["']/.test(html)) throw new Error('Unsupported buildUrl; expected local Build');
  for (const key of ['loaderUrl', 'dataUrl', 'frameworkUrl', 'codeUrl']) {
    const match = html.match(new RegExp(`\\b${key}\\s*[:=]\\s*buildUrl\\s*\\+\\s*["'](/[^"']+)["']`));
    if (!match) throw new Error(`Missing or unsupported ${key}`);
    required.add(safeRelative(`Build${match[1]}`));
  }
  for (const m of html.matchAll(/(?:src|href)\s*=\s*["'](TemplateData\/[^"']+)["']/g)) required.add(safeRelative(m[1]));
  for (const m of html.matchAll(/\bsymbolsUrl\s*:\s*buildUrl\s*\+\s*["'](\/[^"']+)["']/g)) required.add(safeRelative(`Build${m[1]}`));
  const catalogs = {};
  const scan = obj => {
    if (!obj || typeof obj !== 'object') return;
    for (const [key, value] of Object.entries(obj)) {
      if (key === 'bundlePath') required.add(safeRelative(`StreamingAssets/${safeRelative(value)}`));
      else scan(value);
    }
  };
  for (const name of ['aircraft_prefab_catalog.json', 'round_world_catalog.json', 'world_object_catalog.json', 'airrace_rounds.json']) {
    const rel = `StreamingAssets/AirRace/${name}`;
    required.add(rel);
    const json = JSON.parse(await fs.readFile(path.join(root, rel), 'utf8'));
    if (name !== 'airrace_rounds.json' && (typeof json.version !== 'string' || !json.version)) throw new Error(`Missing catalog version: ${name}`);
    if (name === 'aircraft_prefab_catalog.json' && (!Array.isArray(json.prefabs) || !json.prefabs.length)) throw new Error('Empty aircraft prefabs');
    if (name === 'round_world_catalog.json' && (!Array.isArray(json.worlds) || !json.worlds.length)) throw new Error('Empty round worlds');
    if (name === 'airrace_rounds.json' && (!json || typeof json !== 'object' || !Object.keys(json).some(k => /^[1-9][0-9]*$/.test(k)) || Object.entries(json).some(([key, value]) => !/^[1-9][0-9]*$/.test(key) || !value || typeof value !== 'object' || Array.isArray(value)))) throw new Error('Invalid rounds');
    const entries = name === 'aircraft_prefab_catalog.json' ? json.prefabs : name === 'round_world_catalog.json' ? json.worlds : null;
    for (const entry of entries ?? []) {
      if (!entry || typeof entry.prefabName !== 'string' || !entry.prefabName) throw new Error(`Invalid prefab entry: ${name}`);
      safeRelative(entry.bundlePath);
      if (name === 'round_world_catalog.json' && (!Number.isInteger(entry.round) || entry.round < 1)) throw new Error('Invalid world round');
    }
    catalogs[name] = json.version ?? null;
    scan(json);
  }
  for (const rel of required) {
    if (!available.has(rel)) throw new Error(`Missing referenced file: ${rel}`);
    if (!(await fs.stat(path.join(root, rel))).size) throw new Error(`Empty referenced file: ${rel}`);
  }
  const entries = [];
  for (const rel of files) {
    const file = path.join(root, rel);
    const sha256 = await hashFile(file);
    entries.push({ path: rel, bytes: (await fs.stat(file)).size, sha256, httpSha256: /\.(gz|br)$/.test(rel) ? await hashFile(file, true) : sha256 });
  }
  return { game, catalogs, required: [...required].sort(), files: entries };
}
export async function manifest(repo, sourceSha) {
  if (!/^[a-f0-9]{40}$/.test(sourceSha ?? '')) throw new Error('SOURCE_SHA must be an exact repository commit SHA');
  const games = [];
  for (const game of ['airrace3d', 'colony-racer']) games.push(await verifyGame(path.join(repo, 'web/public', game), game));
  return { schema: 1, sourceSha, unitySourceSha: null, unityBuildPerformed: false, games };
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const result = await manifest(process.argv[2] ?? '.', process.env.SOURCE_SHA);
  const output = process.argv[3] ?? 'delivery-manifest.json';
  await fs.writeFile(output, `${JSON.stringify(result, null, 2)}\n`);
  console.log(`Verified ${result.games.length} games; wrote ${output}. Unity source provenance is not inferred.`);
}
