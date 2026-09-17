import { readJSON } from './llm.mjs';
const delay = (ms, _, { signal }) => new Promise((resolve, reject) => {
  signal?.throwIfAborted();
  const abort = () => { clearTimeout(timer); reject(new Error('Place search canceled')); };
  const timer = setTimeout(() => { signal?.removeEventListener('abort', abort); resolve(); }, ms);
  signal?.addEventListener('abort', abort, { once: true });
});

// A configured Nominatim-compatible service (hosted or self-hosted).
// No public geocoder is selected implicitly.
export function createGeocoder({ url, userAgent = 'DioramaCabinet/1.0' }, fetchImpl = fetch) {
  const cache = new Map();
  let queue = Promise.resolve();
  let nextRequest = 0;
  return async (query, radius, signal) => {
    if (!url) throw new Error('Set GEOCODER_URL to a Nominatim-compatible search endpoint on the server.');
    const key = query.trim().toLowerCase();
    const run = queue.catch(() => {}).then(async () => {
      signal?.throwIfAborted();
      const cached = cache.get(key);
      if (cached && cached.expires > Date.now()) return cached.results;
      await delay(Math.max(0, nextRequest - Date.now()), undefined, { signal });
      nextRequest = Date.now() + 1100;
      const endpoint = new URL(url);
      endpoint.searchParams.set('q', query);
      endpoint.searchParams.set('format', 'jsonv2');
      endpoint.searchParams.set('limit', '5');
      const response = await fetchImpl(endpoint, { signal, redirect: 'error', headers: { 'User-Agent': userAgent, Accept: 'application/json' } });
      if (!response.ok) { await response.body?.cancel(); throw new Error(`Place search returned HTTP ${response.status}. Try again later.`); }
      const data = await readJSON(response, 250000);
      if (!Array.isArray(data)) throw new Error('Place search returned an invalid response');
      const results = data.slice(0, 5).flatMap(p => {
        if (!p || ![p.lat, p.lon].every(v => (typeof v === 'number' || typeof v === 'string') && String(v).trim() !== '')) return [];
        const lat = Number(p.lat), lon = Number(p.lon);
        if (typeof p.display_name !== 'string' || !p.display_name || p.lat == null || p.lon == null) return [];
        if (!Number.isFinite(lat) || !Number.isFinite(lon) || Math.abs(lat) > 80 || Math.abs(lon) + 1000 / (111195 * Math.cos(lat * Math.PI / 180)) >= 180) return [];
        return [{ label: p.display_name.slice(0, 500), lat, lon }];
      });
      if (cache.size >= 100) cache.delete(cache.keys().next().value);
      cache.set(key, { expires: Date.now() + 3600000, results });
      return results;
    });
    queue = run;
    return (await run).map(p => ({ ...p, radius }));
  };
}
