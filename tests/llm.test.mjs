import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createLLM } from '../server/llm.mjs';
import { validateScene } from '../server/schema.mjs';
import { createGeocoder } from '../server/geocode.mjs';

export const exampleScene = () => ({ name: 'Violet village', terrain: { half_size: 18, resolution: 32, base_color: [.3, .4, .5], control_points: [], color_zones: [] }, buildings: [{ cx: 0, cz: 0, width: 3, depth: 3, height: 5, color: [.6, .3, .8], shape: 'rect' }], trees: [], water: [], roads: [] });
const completion = value => Response.json({ choices: [{ finish_reason: 'stop', message: { content: typeof value === 'string' ? value : JSON.stringify(value) } }] });

test('generic adapter uses configured endpoint, credentials and schema', async () => {
  const interpret = createLLM({ baseUrl: 'http://localhost:11434/v1/', model: 'local-model', apiKey: 'test-key' }, async (url, options) => {
    assert.equal(String(url), 'http://localhost:11434/v1/chat/completions');
    assert.equal(options.headers.Authorization, 'Bearer test-key');
    const body = JSON.parse(options.body);
    assert.equal(body.model, 'local-model');
    assert.equal(body.response_format.type, 'json_schema');
    assert.equal(body.response_format.json_schema.strict, true);
    return completion(exampleScene());
  });
  assert.equal((await interpret('A violet village', 'scene')).name, 'Violet village');
});

test('one repair fixes invalid geometry, but a second failure stops', async () => {
  let calls = 0;
  const invalid = exampleScene(); invalid.buildings[0].width = 200;
  const interpret = createLLM({ model: 'test' }, async (_, options) => {
    calls++;
    if (calls === 2) assert.match(JSON.parse(options.body).messages.at(-1).content, /width/);
    return completion(calls === 1 ? invalid : exampleScene());
  });
  await interpret('village', 'scene');
  assert.equal(calls, 2);
  calls = 0;
  const broken = createLLM({ model: 'test' }, async () => { calls++; return completion('not JSON'); });
  await assert.rejects(broken('village', 'scene'), /validation/);
  assert.equal(calls, 2);
});

test('JSON and text modes work without native JSON Schema support', async () => {
  for (const outputMode of ['json', 'text']) {
    const interpret = createLLM({ model: 'test', outputMode }, async (_, options) => {
      const body = JSON.parse(options.body);
      assert.deepEqual(body.response_format, outputMode === 'json' ? { type: 'json_object' } : undefined);
      return completion('```json\n{"query":"Tokyo Station, Japan","radius_m":500}\n```');
    });
    assert.equal((await interpret('Tokyo station', 'place')).radius_m, 500);
  }
});

test('HTTP errors, refusals and truncation are not retried or leaked', async () => {
  for (const [response, pattern] of [
    [new Response('secret provider error', { status: 401 }), /HTTP 401/],
    [Response.json({ choices: [{ message: { refusal: 'reason' } }] }), /declined/],
    [Response.json({ choices: [{ finish_reason: 'length' }] }), /did not finish/],
  ]) {
    let calls = 0;
    const interpret = createLLM({ model: 'test' }, async () => { calls++; return response; });
    await assert.rejects(interpret('test', 'scene'), pattern);
    assert.equal(calls, 1);
  }
});

test('validation rejects expensive, out-of-bounds and degenerate scenes', () => {
  for (const mutate of [
    s => { s.terrain.resolution = 100000; },
    s => { s.terrain.control_points = [{ x: 0, z: 0, amplitude: 1, sigma: 0 }]; },
    s => { s.buildings[0].cx = 18; },
    s => { s.buildings[0].color[0] = Infinity; },
    s => { s.roads = [{ points: [[0, 0], [0, 0]], width: 1 }]; },
    s => { s.buildings[0].footprint = [[0, 0]]; },
    s => { s.trees = Array(151).fill({}); },
    s => { s.water = [{ cx: 0, cz: 0, radius_x: 3, radius_z: 2 }]; },
  ]) {
    const scene = exampleScene(); mutate(scene);
    assert.throws(() => validateScene(scene));
  }
});

test('geocoder preserves zero coordinates, returns choices and caches names', async () => {
  let calls = 0;
  const geocode = createGeocoder({ url: 'https://geo.example/search' }, async url => {
    calls++;
    assert.equal(url.searchParams.get('q'), 'Greenwich');
    return Response.json([{ lat: '0', lon: '0', display_name: 'First place' }, { lat: '51.48', lon: '0', display_name: 'Second place' }, { lat: 'bad', lon: '0', display_name: 'Invalid' }]);
  });
  const result = await geocode('Greenwich', 500);
  assert.equal(result.length, 2);
  assert.equal(result[0].lat, 0);
  assert.equal((await geocode('Greenwich', 750))[0].radius, 750);
  assert.equal(calls, 1);
});

test('place interpretation cannot supply invented coordinates and needs geocoder configuration', async () => {
  const interpret = createLLM({ model: 'test' }, async () => completion({ query: 'Tokyo', radius_m: 500, lat: 35, lon: 139 }));
  await assert.rejects(interpret('Tokyo', 'place'), /not supported/);
  await assert.rejects(createGeocoder({})('Tokyo', 500), /GEOCODER_URL/);
});

test('water must have a depression and a valid lake passes', () => {
  const scene=exampleScene();
  scene.water=[{cx:0,cz:5,radius_x:3,radius_z:2}];
  assert.throws(()=>validateScene(scene),/depression/);
  scene.terrain.control_points=[{x:0,z:5,amplitude:-1,sigma:3}];
  assert.equal(validateScene(scene),scene);
});

test('oversized provider output is rejected before JSON parsing', async () => {
  const interpret=createLLM({model:'test'},async()=>new Response('x'.repeat(1000001)));
  await assert.rejects(interpret('village','scene'),/too large/);
});

test('geocoder failure and empty results do not fabricate a place', async () => {
  await assert.rejects(createGeocoder({url:'https://geo.example/search'},async()=>new Response('private error',{status:503}))('Tokyo',500),/HTTP 503/);
  assert.deepEqual(await createGeocoder({url:'https://geo.example/search'},async()=>Response.json([]))('Nowhere',500),[]);
});

test('geocoder rejects empty coordinate strings instead of turning them into zero', async () => {
  const geocode=createGeocoder({url:'https://geo.example/search'},async()=>Response.json([{lat:'',lon:'',display_name:'Broken place'}]));
  assert.deepEqual(await geocode('Broken place',500),[]);
});
