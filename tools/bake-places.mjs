// Pre-builds the places in tools/places.json into web/scenes/places/ with the
// same Overpass query and conversion the live importer uses. Behind an HTTP
// proxy, run with NODE_USE_ENV_PROXY=1 (Node 24+).
//
//   node tools/bake-places.mjs [radius-metres] [place-id ...]
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { fetchOSM, convertOSM } from '../web/osm.mjs';

const root = new URL('../', import.meta.url);
const outDir = new URL('web/scenes/places/', root);
const [radiusArg, ...only] = process.argv.slice(2);
const radius = Number(radiusArg ?? 400);
const all = JSON.parse(await readFile(new URL('tools/places.json', root), 'utf8'));
const chosen = only.length ? all.filter(place => only.includes(place.id)) : all;
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

await mkdir(outDir, { recursive: true });
const manifestURL = new URL('index.json', outDir);
let manifest = [];
try { manifest = JSON.parse(await readFile(manifestURL, 'utf8')); } catch {}

let failed = 0;
for (const [i, place] of chosen.entries()) {
  if (i) await pause(5000); // stay well inside Overpass's per-IP rate limit
  try {
    const data = await fetchOSM(place.lat, place.lon, radius, AbortSignal.timeout(60000));
    const scene = convertOSM(data, { lat: place.lat, lon: place.lon, radius });
    const json = JSON.stringify(scene);
    await writeFile(new URL(`${place.id}.json`, outDir), json);
    manifest = manifest.filter(entry => entry.id !== place.id);
    manifest.push({ ...place, radius });
    console.log(`${place.id}: ${scene.metadata.stats.buildings} buildings, ${(json.length / 1e6).toFixed(1)} MB`);
  } catch (err) {
    failed++;
    console.error(`${place.id}: ${err.message}`);
  }
}
// Keep the picker in the order of tools/places.json.
manifest.sort((a, b) => all.findIndex(p => p.id === a.id) - all.findIndex(p => p.id === b.id));
await writeFile(manifestURL, JSON.stringify(manifest, null, 2) + '\n');
console.log(`${manifest.length} places in web/scenes/places/index.json${failed ? `, ${failed} failed` : ''}`);
process.exitCode = failed ? 1 : 0;
