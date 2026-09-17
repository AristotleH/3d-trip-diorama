import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createLLM } from './llm.mjs';
import { createGeocoder } from './geocode.mjs';

const webRoot = path.resolve(fileURLToPath(new URL('../web/', import.meta.url)));
const types = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json', '.wasm': 'application/wasm', '.css': 'text/css' };
const json = (res, status, body) => { res.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' }); res.end(JSON.stringify(body)); };

export function createServer({ interpret, geocode, timeoutMs = 90000, llmEnabled = true, llmConfigured = true } = {}) {
  let busy = false;
  return http.createServer(async (req, res) => {
    res.setHeader('X-Content-Type-Options', 'nosniff');
    const expectedHost = `127.0.0.1:${req.socket.localPort}`;
    const alternateHost = `localhost:${req.socket.localPort}`;
    if (![expectedHost, alternateHost].includes(req.headers.host)) return json(res, 403, { error: 'Use the localhost address printed by the server.' });
    if (req.headers.origin && req.headers.origin !== `http://${req.headers.host}`) return json(res, 403, { error: 'Cross-origin requests are not allowed.' });
    const pathname = new URL(req.url, 'http://localhost').pathname;
    if (pathname === '/api/config') return json(res, 200, { llmEnabled, llmConfigured });
    if (pathname === '/api/interpret') {
      if (!llmEnabled) return json(res, 403, { error: 'AI generation is disabled on this site.' });
      if (req.method !== 'POST') return json(res, 405, { error: 'Use POST.' });
      if (!req.headers['content-type']?.startsWith('application/json')) return json(res, 415, { error: 'Use JSON.' });
      if (busy) return json(res, 429, { error: 'Another generation is in progress. Try again shortly.' });
      busy = true;
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), timeoutMs);
      const disconnect = () => { if (!res.writableEnded) controller.abort(); };
      res.on('close', disconnect);
      try {
        let bytes = 0;
        const chunks = [];
        for await (const chunk of req) {
          bytes += chunk.length;
          if (bytes > 16000) { json(res, 413, { error: 'Request is too large.' }); return; }
          chunks.push(chunk);
        }
        let input;
        try { input = JSON.parse(Buffer.concat(chunks).toString()); } catch { return json(res, 400, { error: 'Invalid JSON request.' }); }
        if (!input || !['scene', 'place'].includes(input.mode) || typeof input.prompt !== 'string' || !input.prompt.trim() || input.prompt.length > 4000) return json(res, 400, { error: 'Choose a mode and enter 1–4,000 characters.' });
        const result = await interpret(input.prompt.trim(), input.mode, controller.signal);
        if (input.mode === 'scene') json(res, 200, { kind: 'scene', scene: result });
        else if (result.query === 'NEEDS_CLARIFICATION') json(res, 200, { kind: 'clarify', message: 'Specify one place, including its city or country, and an area of 100–1,000 metres radius.' });
        else {
          const candidates = await geocode(result.query, result.radius_m, controller.signal);
          json(res, 200, { kind: 'places', query: result.query, candidates });
        }
      } catch (error) {
        if (!res.destroyed && !res.writableEnded) json(res, 502, { error: controller.signal.aborted ? 'Generation timed out or was canceled. Try a shorter request.' : error.message });
      } finally {
        clearTimeout(timer);
        res.off('close', disconnect);
        busy = false;
      }
      return;
    }
    if (!['GET', 'HEAD'].includes(req.method)) return json(res, 405, { error: 'Method not allowed.' });
    try {
      const relative = decodeURIComponent(pathname === '/' ? '/index.html' : pathname);
      const file = path.resolve(webRoot, `.${relative}`);
      if (!file.startsWith(webRoot + path.sep) || relative.split('/').some(part => part.startsWith('.'))) return json(res, 404, { error: 'Not found.' });
      const data = await readFile(file);
      res.writeHead(200, { 'Content-Type': types[path.extname(file)] || 'application/octet-stream', 'Cache-Control': 'no-cache' });
      res.end(req.method === 'HEAD' ? undefined : data);
    } catch { json(res, 404, { error: 'Not found.' }); }
  });
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const llmEnabled = process.env.LLM_ENABLED !== 'false';
  const interpret = llmEnabled ? createLLM({ baseUrl: process.env.LLM_BASE_URL, model: process.env.LLM_MODEL, apiKey: process.env.LLM_API_KEY, outputMode: process.env.LLM_OUTPUT_MODE }) : undefined;
  const geocode = createGeocoder({ url: process.env.GEOCODER_URL, userAgent: process.env.GEOCODER_USER_AGENT });
  const server = createServer({ interpret, geocode, llmEnabled, llmConfigured: Boolean(process.env.LLM_MODEL) });
  server.requestTimeout = 100000;
  server.headersTimeout = 10000;
  server.listen(Number(process.env.PORT || 8080), '127.0.0.1', () => console.log(`Diorama: http://127.0.0.1:${server.address().port}`));
}
