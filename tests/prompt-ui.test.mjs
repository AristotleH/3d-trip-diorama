import { test } from 'node:test';
import assert from 'node:assert/strict';
import { setupPrompt } from '../web/prompt-ui.mjs';

function harness(fetchImpl) {
  const make = () => ({ children: [], hidden: false, disabled: true, textContent: '', addEventListener(type, fn) { this[type] = fn; }, replaceChildren() { this.children = []; }, append(child) { this.children.push(child); } });
  const elements = Object.fromEntries(['prompt-form', 'prompt-build', 'prompt-cancel', 'place-results'].map(id => [id, make()]));
  elements['prompt-form'].elements = { prompt: { value: 'A violet town' }, mode: { value: 'scene' } };
  globalThis.document = { querySelector: selector => elements[selector.slice(1)], createElement: make };
  let ticket = 0;
  const scenes = [], places = [], statuses = [];
  setupPrompt({ fetchImpl, onStart: () => { const current = ++ticket; return () => current === ticket; }, onScene: s => scenes.push(s), onPlace: p => places.push(p), onStatus: s => statuses.push(s) });
  return { elements, scenes, places, statuses, stale() { ticket++; }, submit: () => elements['prompt-form'].submit({ preventDefault() {} }) };
}

test('prompt creates a scene and restores controls', async () => {
  const h = harness(async (_, options) => {
    assert.equal(JSON.parse(options.body).mode, 'scene');
    return Response.json({ kind: 'scene', scene: { name: 'Violet town' } });
  });
  await h.submit();
  assert.equal(h.scenes[0].name, 'Violet town');
  assert.equal(h.elements['prompt-build'].disabled, false);
  assert.equal(h.elements['prompt-cancel'].hidden, true);
});

test('geocoder matches require selection and carry radius into OSM', async () => {
  const candidate = { label: 'Tokyo Station', lat: 35, lon: 139, radius: 500 };
  const h = harness(async () => Response.json({ kind: 'places', candidates: [candidate] }));
  await h.submit();
  assert.equal(h.places.length, 0);
  h.elements['place-results'].children[0].click();
  assert.deepEqual(h.places, [candidate]);
  assert.equal(h.elements['place-results'].children.length, 0);
});

test('gallery changes suppress pending results and stale candidate clicks', async () => {
  let resolve;
  const h = harness(() => new Promise(r => { resolve = r; }));
  const pending = h.submit();
  h.stale();
  resolve(Response.json({ kind: 'scene', scene: { name: 'Old scene' } }));
  await pending;
  assert.equal(h.scenes.length, 0);
  const choice = harness(async () => Response.json({ kind: 'places', candidates: [{ label: 'Old place', lat: 0, lon: 0, radius: 500 }] }));
  await choice.submit();
  choice.stale();
  choice.elements['place-results'].children[0].click();
  assert.equal(choice.places.length, 0);
});

test('cancel and provider errors preserve scene and restore controls', async () => {
  const h = harness((_, { signal }) => new Promise((resolve, reject) => signal.addEventListener('abort', () => reject(new Error('aborted')))));
  const pending = h.submit();
  h.elements['prompt-cancel'].click();
  await pending;
  assert.equal(h.scenes.length, 0);
  assert.match(h.statuses.at(-1), /canceled/);
  assert.equal(h.elements['prompt-build'].disabled, false);
  const failure = harness(async () => Response.json({ error: 'Service unavailable' }, { status: 502 }));
  await failure.submit();
  assert.equal(failure.statuses.at(-1), 'Service unavailable');
  assert.equal(failure.scenes.length, 0);
});
