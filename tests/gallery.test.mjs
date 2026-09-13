import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { osmHash, parseOSMHash } from '../web/osm-url.mjs';
import { convertOSM, validateCenter } from '../web/osm.mjs';

const source = readFileSync(new URL('../web/index.js', import.meta.url), 'utf8')
  .replace(/^import .*?;$/gm, '');
const tick = () => new Promise(resolve => setImmediate(resolve));
function gallery(fetch, hash = '', realOSMFetch = null) {
  const elements = Object.fromEntries(['scene', 'status', 'description', 'schema', 'osm-credit', 'osm-details', 'osm-form', 'osm-build', 'osm-cancel', 'osm-close'].map(id => [id, {
    value: 'default', disabled: true, textContent: '', addEventListener(type, fn) { this[type] = fn; },
  }]));
  const loaded = [];
  const imports = [], urls = [], events = {};
  let osmCallbacks;
  const location = { hash };
  elements['osm-form'].elements = {latitude:{value:''},longitude:{value:''},radius:{value:''}};
  const context = vm.createContext({
    fetchOSM: realOSMFetch, convertOSM, validateCenter, AbortController, setTimeout, clearTimeout,
    osmHash, parseOSMHash,
    setupOSM: callbacks => { osmCallbacks = callbacks; return {load: async params => imports.push(params)}; },
    init: async () => {}, init_diorama: async () => {}, load_scene: json => loaded.push(json),
    document: { querySelector: id => elements[id.slice(1)] },
    location, history: { replaceState(a, b, url) { urls.push(url); } }, fetch,
    window: { addEventListener(type, fn) { events[type] = fn; } },
    URL, Blob, Option: function(text, value) { this.value = value; },
    console: { error() {} },
  });
  if (realOSMFetch) {
    const ui = readFileSync(new URL('../web/osm-ui.mjs', import.meta.url), 'utf8')
      .replace(/^import .*$/gm, '').replace('export function', 'function');
    vm.runInContext(ui, context);
  }
  vm.runInContext(source, context);
  elements.scene.querySelector = () => true;
  return { elements, loaded, imports, urls,
    showImport(scene) { osmCallbacks.onScene(scene); },
    navigate(hash) { location.hash = hash; return events.hashchange(); },
    select(id) { elements.scene.value = id; elements.scene.change(); } };
}

test('exact Mission Bay URL fetches, converts and installs a scene without a click', async () => {
  const calls = [];
  const lat=37.77453, lon=-122.389813;
  const g = gallery(() => {throw new Error('no default scene request expected');},
    '#osm?lat=37.77453&lon=-122.389813&radius=1000',
    async (...args) => {
      calls.push(args);
      return {elements:[{type:'way',id:1,tags:{building:'yes'},geometry:[
        {lat,lon},{lat,lon:lon+.0001},{lat:lat+.0001,lon:lon+.0001},{lat:lat+.0001,lon},{lat,lon}
      ]}]};
    });
  await tick();
  assert.deepEqual(calls[0].slice(0,3),[lat,lon,1000]);
  assert.equal(g.loaded.length,1);
  const scene = JSON.parse(g.loaded[0]);
  assert.deepEqual(scene.metadata.center,{lat,lon});
  assert.equal(scene.buildings.length,1);
  assert.equal(g.elements.scene.value,'osm');
  assert.equal(g.elements['osm-form'].elements.radius.value,'1000');
  assert.equal(g.elements.status.textContent,'');
});

test('an automatic import failure is visible and leaves the form ready to retry', async () => {
  const g = gallery(() => {}, '#osm?lat=37.77453&lon=-122.389813&radius=1000',
    async () => {throw new Error('OSM service is busy');});
  await tick();
  assert.equal(g.loaded.length,0);
  assert.equal(g.elements['osm-details'].open,true);
  assert.equal(g.elements['osm-build'].disabled,false);
  assert.match(g.elements.status.textContent,/OSM service is busy/);
  assert.match(g.elements.description.textContent,/could not load/);
});

test('OSM links load on startup and when the fragment changes', async () => {
  const g = gallery(() => { throw new Error('unexpected gallery request'); }, '#osm?lat=37.77453&lon=-122.389813&radius=1000');
  await tick();
  assert.deepEqual(g.imports, [{lat:37.77453,lon:-122.389813,radius:1000}]);
  await g.navigate('#osm?lat=0&lon=0&radius=500');
  assert.deepEqual(g.imports[1], {lat:0,lon:0,radius:500});
});

test('invalid OSM links show an input error without fetching', async () => {
  const g = gallery(() => { throw new Error('unexpected request'); }, '#osm?lat=90&lon=0&radius=500');
  await tick();
  assert.equal(g.imports.length, 0);
  assert.match(g.elements.status.textContent, /Could not load OSM link/);
});

test('successful imports put their actual coordinates and radius in the URL', async () => {
  const g = gallery(() => {});
  await tick();
  g.showImport({metadata:{center:{lat:37.77453,lon:-122.389813},radius_m:1000,stats:{}}});
  assert.equal(g.urls.at(-1), '#osm?lat=37.77453&lon=-122.389813&radius=1000');
});

test('switches worlds and updates the downloadable JSON', async () => {
  const g = gallery(async () => ({ ok: true, text: async () => '{"name":"Ice"}' }));
  await tick();
  assert.equal(g.elements.scene.disabled, false);
  g.select('glacier-organ');
  await tick();
  assert.deepEqual(g.loaded, ['{"name":"Ice"}']);
  assert.equal(g.elements.schema.download, 'glacier-organ.json');
});

test('a slow earlier request cannot replace the latest selected world', async () => {
  const pending = [];
  const g = gallery(() => new Promise(resolve => pending.push(resolve)));
  await tick();
  g.select('violet-singularity');
  g.select('glacier-organ');
  pending[1]({ ok: true, text: async () => 'new' });
  await tick();
  pending[0]({ ok: true, text: async () => 'old' });
  await tick();
  assert.deepEqual(g.loaded, ['new']);
  assert.equal(g.elements.scene.value, 'glacier-organ');
});

test('download failure keeps the currently displayed world selected', async () => {
  const g = gallery(async () => ({ ok: false, status: 404 }));
  await tick();
  g.select('glacier-organ');
  await tick();
  assert.equal(g.elements.scene.value, 'default');
  assert.deepEqual(g.loaded, []);
  assert.match(g.elements.status.textContent, /Could not load/);
});
