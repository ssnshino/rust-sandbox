import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { safeRelative } from './verify-game-delivery.mjs';
export async function smoke(manifest, base, repo) {
  if (manifest?.schema !== 1 || !/^[a-f0-9]{40}$/.test(manifest.sourceSha ?? '') || !Array.isArray(manifest.games) || manifest.games.length !== 2 || manifest.games.map(g => g.game).sort().join(',') !== 'airrace3d,colony-racer') throw new Error('Invalid two-game manifest');
  for (const game of manifest.games) {
    if (!Array.isArray(game.files) || !game.files.length || !Array.isArray(game.required) || !game.required.includes('index.html')) throw new Error('Incomplete game manifest');
    const names = new Set();
    for (const file of game.files) { safeRelative(file.path); if (names.has(file.path) || !/^[a-f0-9]{64}$/.test(file.httpSha256 ?? '')) throw new Error('Invalid file hash or duplicate'); names.add(file.path); }
    if (game.required.some(p => !names.has(p))) throw new Error('Missing required inventory entry');
  }
  const origin = new URL(base);
  if (origin.pathname !== '/' || origin.search || origin.hash || origin.username || origin.password || !['http:', 'https:'].includes(origin.protocol)) throw new Error('Expected plain origin URL');
  async function get(rel) {
    const url = new URL(rel, origin);
    url.searchParams.set('delivery', manifest.sourceSha);
    const response = await fetch(url, { signal: AbortSignal.timeout(60000), redirect: 'error', cache: 'no-store' });
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${rel}`);
    return response;
  }
  let ready = false;
  for (let attempt = 0; attempt < 12; attempt++) {
    try {
      const response = await fetch(new URL('/healthz', origin), { signal: AbortSignal.timeout(5000), redirect: 'error', cache: 'no-store' });
      const health = response.ok ? await response.json() : null;
      ready = health?.ok === true && health?.service === 'unity-rust-games-web';
      if (ready) break;
    } catch { /* Container may still be compiling the Node server. */ }
    if (attempt < 11) await new Promise(resolve => setTimeout(resolve, 5000));
  }
  if (!ready) throw new Error('Expected health service not ready within bounded startup window');
  for (const game of manifest.games) {
    safeRelative(game.game);
    for (const file of game.files) {
      safeRelative(file.path);
      const r = await get(`/${game.game}/${file.path}`);
      if (file.path.endsWith('.gz') && r.headers.get('content-encoding') !== 'gzip') throw new Error(`gzip header missing: ${file.path}`);
      if (file.path.endsWith('.br') && r.headers.get('content-encoding') !== 'br') throw new Error(`brotli header missing: ${file.path}`);
      if (/\.wasm(?:\.gz|\.br)?$/.test(file.path) && !r.headers.get('content-type')?.startsWith('application/wasm')) throw new Error(`wasm MIME wrong: ${file.path}`);
      if (/\.js(?:\.gz|\.br)?$/.test(file.path) && !/javascript/.test(r.headers.get('content-type') ?? '')) throw new Error(`JS MIME wrong: ${file.path}`);
      const hash = createHash('sha256');
      for await (const chunk of r.body) hash.update(chunk);
      if (hash.digest('hex') !== file.httpSha256) throw new Error(`Public artifact mismatch: ${game.game}/${file.path}`);
    }
    for (const [endpoint, file] of [['round-world-catalog', 'round_world_catalog.json'], ['aircraft-prefab-catalog', 'aircraft_prefab_catalog.json'], ['world-object-catalog', 'world_object_catalog.json']]) {
      const actual = await (await get(`/api/${game.game}/${endpoint}`)).json();
      const expected = JSON.parse(await fs.readFile(path.join(repo, 'web/public', game.game, 'StreamingAssets/AirRace', file), 'utf8'));
      if (!isDeepStrictEqual(actual, expected)) throw new Error(`API catalog mismatch: ${game.game}/${endpoint}`);
    }
  }
  console.log('HTTP/artifact/API smoke passed. Browser/WebGL runtime QA is a separate, unperformed gate.');
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await smoke(JSON.parse(await fs.readFile(process.argv[2], 'utf8')), process.argv[3], process.argv[4] ?? '.');
