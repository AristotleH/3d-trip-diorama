import { test } from 'node:test';
import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createServer } from '../server/index.mjs';

async function serve(t, services) {
  const server = createServer(services);
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  t.after(() => { server.closeAllConnections(); server.close(); });
  const base = `http://127.0.0.1:${server.address().port}`;
  return { base, post: body => fetch(`${base}/api/interpret`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }) };
}

test('server serves the app, keeps source/secrets private and rejects foreign origins', async t => {
  const { base } = await serve(t, {});
  assert.match(await (await fetch(base)).text(), /prompt-form/);
  for (const pathname of ['/server/index.mjs', '/.env', '/Cargo.toml']) assert.equal((await fetch(base + pathname)).status, 404);
  assert.equal((await fetch(base + '/api/interpret', { method: 'POST', headers: { Origin: 'https://foreign.example' } })).status, 403);
});

test('plaintext requests route to scene and place services with input validation', async t => {
  let calls = 0;
  const { post } = await serve(t, {
    interpret: async (prompt, mode) => { calls++; return mode === 'scene' ? { name: prompt } : { query: 'Tokyo Station', radius_m: 500 }; },
    geocode: async (query, radius) => [{ label: query, lat: 35, lon: 139, radius }],
  });
  assert.deepEqual(await (await post({ prompt: 'Violet city', mode: 'scene' })).json(), { kind: 'scene', scene: { name: 'Violet city' } });
  const places = await (await post({ prompt: 'Tokyo', mode: 'place' })).json();
  assert.equal(places.candidates[0].radius, 500);
  assert.equal((await post({ prompt: '', mode: 'scene' })).status, 400);
  assert.equal((await post({ prompt: 'test', mode: 'invalid' })).status, 400);
  assert.equal(calls, 2);
});

test('clarification does not invoke geocoding', async t => {
  const { post } = await serve(t, { interpret: async () => ({ query: 'NEEDS_CLARIFICATION' }), geocode: () => assert.fail('unexpected geocoder') });
  assert.equal((await (await post({ mode: 'place', prompt: 'somewhere' })).json()).kind, 'clarify');
});

test('timeout cancels upstream and releases the single-request slot', async t => {
  const { post } = await serve(t, { timeoutMs: 30, interpret: (_, __, signal) => new Promise((resolve, reject) => signal.addEventListener('abort', () => reject(new Error('aborted')))) });
  for (let i = 0; i < 2; i++) {
    const response = await post({ mode: 'scene', prompt: 'slow model' });
    assert.equal(response.status, 502);
    assert.match((await response.json()).error, /timed out/);
  }
});

test('concurrent requests are rejected and the slot is released after success', async t => {
  let finish, started;
  let calls = 0;
  const ready = new Promise(r => { started = r; });
  const { post } = await serve(t, { interpret: () => ++calls === 1 ? new Promise(resolve => { finish = resolve; started(); }) : Promise.resolve({name:'next'}) });
  const first = post({mode:'scene',prompt:'first'});
  await ready;
  assert.equal((await post({mode:'scene',prompt:'second'})).status, 429);
  finish({name:'done'});
  assert.equal((await first).status, 200);
  assert.equal((await post({mode:'scene',prompt:'third'})).status, 200);
});

test('invalid JSON, oversized bodies and wrong methods never reach the model', async t => {
  const {base, post} = await serve(t, {interpret:()=>assert.fail('model should not run')});
  assert.equal((await fetch(base+'/api/interpret')).status,405);
  assert.equal((await fetch(base+'/api/interpret',{method:'POST',body:'hello'})).status,415);
  assert.equal((await fetch(base+'/api/interpret',{method:'POST',headers:{'Content-Type':'application/json'},body:'{'})).status,400);
  assert.equal((await post({mode:'scene',prompt:'x'.repeat(17000)})).status,413);
});

test('client disconnect aborts the model request', async t => {
  let started, canceled;
  const ready = new Promise(r=>{started=r;});
  const stopped = new Promise(r=>{canceled=r;});
  const {base} = await serve(t,{interpret:(_,__,signal)=>new Promise((resolve,reject)=>{
    signal.addEventListener('abort',()=>{canceled();reject(new Error('aborted'));}); started();
  })});
  const controller = new AbortController();
  const request = fetch(base+'/api/interpret',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({mode:'scene',prompt:'cancel me'}),signal:controller.signal});
  await ready;
  controller.abort();
  await assert.rejects(request);
  await Promise.race([stopped, new Promise((_,reject)=>{const timer=setTimeout(()=>reject(new Error('upstream was not canceled')),1000);timer.unref();})]);
});

test('manual-only server blocks LLM calls but serves the gallery',async t=>{
  const {base,post}=await serve(t,{llmEnabled:false,llmConfigured:false,interpret:()=>assert.fail('LLM must not be called')});
  assert.equal((await fetch(base)).status,200);
  assert.deepEqual(await (await fetch(base+'/api/config')).json(),{llmEnabled:false,llmConfigured:false});
  assert.equal((await post({mode:'scene',prompt:'village'})).status,403);
});
