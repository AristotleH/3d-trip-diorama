import { setupOSM } from './osm-ui.mjs?v=coverage-1';
import { osmHash, parseOSMHash } from './osm-url.mjs?v=coverage-1';
import init, { init_diorama, live_osm_enabled, load_scene } from './pkg/diorama_app.js?v=coverage-1';

const descriptions = {
    default: 'A miniature town with a river and pastel rooftops.',
    'violet-singularity': 'Concentric spires crowd a violet crater and its impossibly blue lagoon.',
    'marshmallow-megalopolis': 'A candy-layered planet of sherbet buildings and giant spun-sugar trees.',
    'glacier-organ': 'Two ranks of icy towers rise like organ pipes above a glacial channel.',
};
const picker = document.querySelector('#scene');
const status = document.querySelector('#status');
const description = document.querySelector('#description');
const schema = document.querySelector('#schema');
let active = 'default';
let places = {};
let request = 0;
let imported = null;
let importedURL = null;
const credit = document.querySelector('#osm-credit');

async function switchScene(id) {
    if (id === 'osm' && imported) {
        ++request;
        showImported(imported);
        return;
    }
    const place = Object.hasOwn(places, id) ? places[id] : null;
    if (!place && !Object.hasOwn(descriptions, id)) return;
    const ticket = ++request;
    status.textContent = 'Loading world…';
    try {
        const url = place ? `./scenes/places/${id}.json` : `./scenes/${id}.json`;
        const response = await fetch(url);
        if (!response.ok) throw new Error(`Scene download failed (${response.status})`);
        const json = await response.text();
        if (ticket !== request) return;
        load_scene(json);
        credit.hidden = !place;
        active = id;
        picker.value = id;
        description.textContent = place ? `${place.name}. ${describeOSM(JSON.parse(json))}` : descriptions[id];
        schema.href = url;
        schema.download = `${id}.json`;
        history.replaceState(null, '', `#${id}`);
        status.textContent = '';
    } catch (err) {
        if (ticket !== request) return;
        picker.value = active;
        status.textContent = 'Could not load that world. Please try again.';
        console.error(err);
    }
}

function showImported(scene) {
    load_scene(JSON.stringify(scene));
    imported = scene;
    if (importedURL) URL.revokeObjectURL(importedURL);
    importedURL = URL.createObjectURL(new Blob([JSON.stringify(scene, null, 2)], { type: 'application/json' }));
    if (!picker.querySelector('option[value="osm"]')) picker.add(new Option('Imported OSM place', 'osm'));
    active = 'osm'; picker.value = 'osm';
    schema.href = importedURL; schema.download = 'osm-diorama.json';
    credit.hidden = false;
    description.textContent = describeOSM(scene);
    status.textContent = '';
    history.replaceState(null, '', osmHash({...scene.metadata.center, radius: scene.metadata.radius_m}));
}

function describeOSM(scene) {
    const s = scene.metadata.stats;
    const labels={geometry_limit:'detail limit',building_limit:'building limit',road_limit:'road limit',surface_limit:'surface limit',invalid_geometry:'invalid geometry',outside_crop:'outside crop',unsupported_road:'unsupported roads'};
    const reasons=Object.entries(s.omitted_by_reason||{}).map(([reason,count])=>`${count} ${labels[reason]||reason}`).join(', ');
    return `${s.buildings} building pieces (${s.estimated_heights} estimated heights), ${s.roads} road segments, ${s.water} inland water areas, ${s.parks} parks. ${s.coastal_water ? 'Coastal water included. ' : ''}${s.courtyard_buildings ? `${s.courtyard_buildings} courtyard buildings. ` : ''}${s.skipped} features omitted${reasons ? `: ${reasons}` : ''}. Flat terrain.`;
}

// Pre-built OpenStreetMap places from tools/bake-places.mjs. The list is optional.
async function loadPlaces() {
    try {
        const response = await fetch('./scenes/places/index.json');
        if (!response.ok) return;
        const list = await response.json();
        const group = document.createElement('optgroup');
        group.label = 'Real places (OpenStreetMap)';
        for (const place of list) {
            places[place.id] = place;
            group.append(new Option(place.name, place.id));
        }
        if (list.length) picker.append(group);
    } catch (err) {
        console.error('Sample places unavailable:', err);
    }
}

async function run() {
    await init({ module_or_path: './pkg/diorama_app_bg.wasm?v=coverage-1' });
    await init_diorama();
    const liveOSM = live_osm_enabled();
    if (!liveOSM) document.querySelector('#osm-details').hidden = true;
    await loadPlaces();
    const osm = liveOSM && setupOSM({
        onStart() { const ticket = ++request; status.textContent = 'Fetching OpenStreetMap…'; return () => ticket === request; },
        onScene: showImported,
        onError(message) {
            status.textContent = message;
            if (description.textContent.startsWith('Loading OSM place'))
                description.textContent = 'The linked place could not load. Check the coordinates and use Build this place to retry.';
        },
    });
    picker.disabled = false;
    status.textContent = '';
    picker.addEventListener('change', () => switchScene(picker.value));
    async function loadFromURL() {
        try {
            const params = osm && parseOSMHash(location.hash);
            if (params) {
                document.querySelector('#osm-details').open = true;
                description.textContent = `Loading OSM place at ${params.lat}, ${params.lon} (${params.radius} m radius)…`;
                await osm.load(params);
                return;
            }
            const requested = location.hash.slice(1);
            if (requested === 'osm' && osm) {
                ++request;
                document.querySelector('#osm-details').open = true;
                status.textContent = 'Enter coordinates to build a place.';
            } else if (Object.hasOwn(descriptions, requested || 'default') || Object.hasOwn(places, requested)) {
                await switchScene(requested || 'default');
            }
        } catch (error) {
            ++request;
            status.textContent = `Could not load OSM link: ${error.message}`;
        }
    }
    window.addEventListener('hashchange', loadFromURL);
    if (location.hash) await loadFromURL();
}

run().catch(err => {
    status.textContent = 'The 3D renderer could not start. Try opening this page in a browser with WebGPU support.';
    console.error('Failed to start diorama:', err);
});
