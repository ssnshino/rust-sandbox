import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { gzipSync } from 'node:zlib';
import { verifyGame, safeRelative, manifest } from '../scripts/verify-game-delivery.mjs';
export async function fixture() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'game-delivery-'));
  const put = async (rel, value) => { await fs.mkdir(path.dirname(path.join(root, rel)), { recursive: true }); await fs.writeFile(path.join(root, rel), value); };
  await put('index.html', 'var buildUrl="Build";var loaderUrl=buildUrl+"/a.loader.js"; const c={dataUrl:buildUrl+"/a.data.gz",frameworkUrl:buildUrl+"/a.framework.js.gz",codeUrl:buildUrl+"/a.wasm.gz"};');
  await put('Build/a.loader.js', 'loader');
  for (const ext of ['data', 'framework.js', 'wasm']) await put(`Build/a.${ext}.gz`, gzipSync(Buffer.from(ext)));
  for (const [name, obj] of Object.entries({aircraft_prefab_catalog:{version:'v1',prefabs:[{bundlePath:'AirRace/Bundles/a',prefabName:'Aircraft'}]},round_world_catalog:{version:'v1',worlds:[{bundlePath:'AirRace/Bundles/a',prefabName:'World',round:1}]},world_object_catalog:{version:'v1',objects:[]},airrace_rounds:{1:{gates:[]}}})) await put(`StreamingAssets/AirRace/${name}.json`, JSON.stringify(obj));
  await put('StreamingAssets/AirRace/Bundles/a', 'bundle');
  return {root,put};
}
test('valid package hashes compressed and decompressed bytes', async () => {const {root}=await fixture();const m=await verifyGame(root,'airrace3d');assert.equal(m.files.length,10);const w=m.files.find(x=>x.path.endsWith('wasm.gz'));assert.notEqual(w.sha256,w.httpSha256);});
test('missing bundle fails closed',async()=>{const {root}=await fixture();await fs.unlink(path.join(root,'StreamingAssets/AirRace/Bundles/a'));await assert.rejects(verifyGame(root,'airrace3d'),/Missing referenced/);});
test('unsafe paths rejected',()=>{for(const p of ['../a','/a','a/../b','a\\b','https://x/a','a%2fb','a?x','a//b'])assert.throws(()=>safeRelative(p));});
test('symlink escape rejected',async()=>{const {root}=await fixture();await fs.symlink('/etc/passwd',path.join(root,'escape'));await assert.rejects(verifyGame(root,'airrace3d'),/Symlink/);});
test('malformed catalog rejected',async()=>{const {root,put}=await fixture();await put('StreamingAssets/AirRace/round_world_catalog.json','{');await assert.rejects(verifyGame(root,'airrace3d'));});
test('unsupported HTML fails rather than evaluates JS',async()=>{const {root,put}=await fixture();await put('index.html','var buildUrl="https://evil.example";');await assert.rejects(verifyGame(root,'airrace3d'),/Unsupported/);});
test('corrupt gzip rejected',async()=>{const {root,put}=await fixture();await put('Build/a.wasm.gz','bad');await assert.rejects(verifyGame(root,'airrace3d'));});
test('source SHA is mandatory',async()=>{await assert.rejects(manifest('.',undefined),/SOURCE_SHA/);});

import { createServer } from 'node:http';
import { smoke } from '../scripts/smoke-game-delivery.mjs';
async function smokeFixture() {
  const {root}=await fixture();
  const repo=await fs.mkdtemp(path.join(os.tmpdir(),'game-smoke-'));
  const target=path.join(repo,'web/public/airrace3d');await fs.mkdir(path.dirname(target),{recursive:true});await fs.cp(root,target,{recursive:true});
  await fs.cp(root,path.join(repo,'web/public/colony-racer'),{recursive:true});
  const m={schema:1,sourceSha:'a'.repeat(40),games:[await verifyGame(target,'airrace3d'),await verifyGame(target,'colony-racer')]};
  let corrupt=false, badHeader=false;
  const server=createServer(async(req,res)=>{
    try {
      const url=new URL(req.url,'http://localhost');
      if(url.pathname==='/healthz'){res.setHeader('content-type','application/json');res.end(JSON.stringify({ok:true,service:'unity-rust-games-web'}));return;}
      if(/^\/api\/(airrace3d|colony-racer)\//.test(url.pathname)){
        const names={'round-world-catalog':'round_world_catalog.json','aircraft-prefab-catalog':'aircraft_prefab_catalog.json','world-object-catalog':'world_object_catalog.json'};
        const name=names[url.pathname.split('/').at(-1)];res.setHeader('content-type','application/json');res.end(await fs.readFile(path.join(target,'StreamingAssets/AirRace',name)));return;
      }
      const rel=url.pathname.replace(/^\/(airrace3d|colony-racer)\//,'');
      if(rel.endsWith('.gz')&&!badHeader)res.setHeader('content-encoding','gzip');
      res.setHeader('content-type',rel.includes('.wasm')?'application/wasm':rel.includes('.js')?'application/javascript':'application/octet-stream');
      res.end(corrupt&&rel==='Build/a.loader.js'?'wrong':await fs.readFile(path.join(target,rel)));
    }catch{res.statusCode=404;res.end('not found');}
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  return {m,repo,url:`http://127.0.0.1:${server.address().port}`,close:()=>new Promise(resolve=>server.close(resolve)),corrupt:()=>{corrupt=true;},badHeader:()=>{badHeader=true;}};
}
test('HTTP smoke verifies gzip decoded hash and API equivalence',async()=>{const s=await smokeFixture();try{await smoke(s.m,s.url,s.repo);}finally{await s.close();}});
test('HTTP smoke rejects stale public artifact',async()=>{const s=await smokeFixture();s.corrupt();try{await assert.rejects(smoke(s.m,s.url,s.repo),/mismatch/);}finally{await s.close();}});
test('HTTP smoke rejects missing gzip header',async()=>{const s=await smokeFixture();s.badHeader();try{await assert.rejects(smoke(s.m,s.url,s.repo),/gzip header/);}finally{await s.close();}});

test('empty smoke manifest rejected',async()=>{await assert.rejects(smoke({schema:1,sourceSha:'a'.repeat(40),games:[]},'http://127.0.0.1','.'),/two-game/);});
test('catalog entries must contain required bundle and prefab',async()=>{const {root,put}=await fixture();await put('StreamingAssets/AirRace/aircraft_prefab_catalog.json',JSON.stringify({version:'v1',prefabs:[{}]}));await assert.rejects(verifyGame(root,'airrace3d'),/Invalid prefab/);});
test('null round rejected',async()=>{const {root,put}=await fixture();await put('StreamingAssets/AirRace/airrace_rounds.json',JSON.stringify({1:null}));await assert.rejects(verifyGame(root,'airrace3d'),/Invalid rounds/);});

test('game root symlink rejected',async()=>{const {root}=await fixture();const parent=await fs.mkdtemp(path.join(os.tmpdir(),'root-symlink-'));const link=path.join(parent,'linked');await fs.symlink(root,link);await assert.rejects(verifyGame(link,'airrace3d'),/root symlink/);});

import { spawnSync } from 'node:child_process';
test('deployment compose guard accepts only reviewed target shape', async()=>{
  const shell=await fs.readFile(new URL('../scripts/deploy-unity-games-dev.sh',import.meta.url),'utf8');
  const code=shell.match(/python3 -c '([\s\S]*?)' "\$repo"/)[1];
  const root='/home/test/games';
  const config={services:{'airrace-web':{container_name:'ayano-lab-unity-rust-games-web',environment:{VIRTUAL_HOST:'airrace-web.wos.ktsys.jp,unity-games.wos.ktsys.jp',VIRTUAL_PORT:'3000'},build:{context:root,dockerfile:'web/Dockerfile'},working_dir:'/workspace/web',command:['sh','-lc','npm run build && npm run start'],ports:[{published:'18093',target:3000,protocol:'tcp'}],volumes:[{type:'bind',source:root+'/web',target:'/workspace/web'},{type:'bind',source:root+'/app/src/airrace',target:'/workspace/app/src/airrace',read_only:true},{type:'volume',target:'/workspace/web/node_modules'}],networks:{default:null,'wos-proxy-network':null}}},networks:{'wos-proxy-network':{external:true}}};
  const run=c=>spawnSync('python3',['-c',code,root],{input:JSON.stringify(c),encoding:'utf8'}).status;
  assert.equal(run(config),0);
  for(const mutate of [c=>c.services['airrace-web'].environment.VIRTUAL_HOST+=',games.lab.ktsys.jp',c=>c.services['airrace-web'].privileged=true,c=>c.services['airrace-web'].volumes[0].source='/etc',c=>c.services['airrace-web'].ports[0].published='80',c=>c.services['airrace-web'].build.dockerfile='other/Dockerfile']){const c=structuredClone(config);mutate(c);assert.notEqual(run(c),0);}
});
