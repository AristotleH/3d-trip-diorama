// Built with schema.mjs, llm.mjs, geocode.mjs and embedded web assets.
// Sites supplies access control before requests reach this private Worker.
let busy = false;
let geocoder;
let geocoderConfig;
export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const reply = (status, body) => Response.json(body, { status, headers: { 'Cache-Control': 'no-store' } });
    if (url.pathname === '/api/config') return reply(200, { llmEnabled: env.LLM_ENABLED !== 'false', llmConfigured: Boolean(env.LLM_MODEL) });
    if (url.pathname === '/api/interpret') {
      if (env.LLM_ENABLED === 'false') return reply(403, { error: 'AI generation is disabled on this site.' });
      if (request.method !== 'POST') return reply(405, { error: 'Use POST.' });
      if (request.headers.get('Origin') && request.headers.get('Origin') !== url.origin) return reply(403, { error: 'Cross-origin requests are not allowed.' });
      if (!request.headers.get('Content-Type')?.startsWith('application/json')) return reply(415, { error: 'Use JSON.' });
      if (!env.LLM_MODEL) return reply(503, { error: 'AI generation is not configured for this site yet.' });
      if (busy) return reply(429, { error: 'Another generation is in progress. Try again shortly.' });
      busy = true;
      const signal = AbortSignal.any([request.signal, AbortSignal.timeout(90000)]);
      try {
        let input;
        try { input = await readJSON(request, 16000); } catch { return reply(400, { error: 'Invalid or oversized JSON request.' }); }
        if (!input || !['scene', 'place'].includes(input.mode) || typeof input.prompt !== 'string' || !input.prompt.trim() || input.prompt.length > 4000) return reply(400, { error: 'Choose a mode and enter 1–4,000 characters.' });
        const interpret = createLLM({ baseUrl: env.LLM_BASE_URL, apiKey: env.LLM_API_KEY, model: env.LLM_MODEL, outputMode: env.LLM_OUTPUT_MODE });
        const result = await interpret(input.prompt, input.mode, signal);
        if (input.mode === 'scene') return reply(200, { kind: 'scene', scene: result });
        if (result.query === 'NEEDS_CLARIFICATION') return reply(200, { kind: 'clarify', message: 'Specify one place, including its city or country, and a radius of 100–1,000 metres.' });
        const config = JSON.stringify([env.GEOCODER_URL, env.GEOCODER_USER_AGENT]);
        if (config !== geocoderConfig) {
          geocoder = createGeocoder({ url: env.GEOCODER_URL, userAgent: env.GEOCODER_USER_AGENT });
          geocoderConfig = config;
        }
        return reply(200, { kind: 'places', query: result.query, candidates: await geocoder(result.query, result.radius_m, signal) });
      } catch (error) { return reply(502, { error: signal.aborted ? 'Generation timed out or was canceled.' : error.message }); }
      finally { busy = false; }
    }
    if (!['GET', 'HEAD'].includes(request.method)) return reply(405, { error: 'Method not allowed.' });
    const asset = SITE_ASSETS[url.pathname === '/' ? '/index.html' : url.pathname];
    if (!asset) return reply(404, { error: 'Not found.' });
    const bytes = Uint8Array.from(atob(asset.body), c => c.charCodeAt(0));
    return new Response(request.method === 'HEAD' ? null : bytes, { headers: { 'Content-Type': asset.type, 'Cache-Control': 'no-cache', 'X-Content-Type-Options': 'nosniff' } });
  },
};
