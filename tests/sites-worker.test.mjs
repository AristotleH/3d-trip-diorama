import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const source=await readFile(new URL('../server/sites-worker.mjs',import.meta.url),'utf8');
const {default:worker}=await import('data:text/javascript;base64,'+Buffer.from('const SITE_ASSETS={"/index.html":{body:"aGVsbG8=",type:"text/html"}};\n'+source).toString('base64'));
test('hosted manual mode serves assets and refuses model requests',async()=>{
  const env={LLM_ENABLED:'false'};
  assert.equal(await (await worker.fetch(new Request('https://site.test/'),env)).text(),'hello');
  assert.equal((await worker.fetch(new Request('https://site.test/api/interpret',{method:'POST'}),env)).status,403);
});
test('hosted AI mode reports missing configuration and never pretends to generate',async()=>{
  const env={};
  const config=await (await worker.fetch(new Request('https://site.test/api/config'),env)).json();
  assert.deepEqual(config,{llmEnabled:true,llmConfigured:false});
  assert.equal((await worker.fetch(new Request('https://site.test/api/interpret',{method:'POST',headers:{'Content-Type':'application/json'}}),env)).status,503);
});
