export function setupPrompt({ onStart, onScene, onPlace, onStatus, fetchImpl = fetch }) {
  const form = document.querySelector('#prompt-form');
  if (!form) return; // Allows embedding just the original gallery.
  const build = document.querySelector('#prompt-build');
  const cancel = document.querySelector('#prompt-cancel');
  const results = document.querySelector('#place-results');
  let controller;
  build.disabled = false;
  form.addEventListener('submit', async event => {
    event.preventDefault();
    controller?.abort();
    const current = new AbortController();
    controller = current;
    const isCurrent = onStart();
    const valid = () => !current.signal.aborted && isCurrent();
    results.replaceChildren();
    build.disabled = true;
    cancel.hidden = false;
    try {
      const response = await fetchImpl('/api/interpret', {
        method: 'POST', headers: { 'Content-Type': 'application/json' }, signal: current.signal,
        body: JSON.stringify({ prompt: form.elements.prompt.value, mode: form.elements.mode.value }),
      });
      if (!valid()) return;
      if (response.status === 404 || response.status === 501) throw new Error('Start the Node server to use plaintext generation.');
      const result = await response.json();
      if (!valid()) return;
      if (!response.ok) throw new Error(result.error || 'Generation failed.');
      if (result.kind === 'scene') onScene(result.scene);
      else if (result.kind === 'clarify') onStatus(result.message);
      else if (result.kind === 'places') {
        onStatus(result.candidates.length ? 'Choose a place to build.' : 'No matching place found. Add a city or country and try again.');
        for (const candidate of result.candidates) {
          const button = document.createElement('button');
          button.type = 'button';
          button.textContent = `${candidate.label} · ${candidate.radius} m radius`;
          button.addEventListener('click', () => {
            if (!valid()) { results.replaceChildren(); return; }
            results.replaceChildren();
            onPlace(candidate);
          });
          results.append(button);
        }
        if (result.candidates.length) {
          const credit = document.createElement('a');
          credit.href = 'https://www.openstreetmap.org/copyright';
          credit.textContent = 'Place search © OpenStreetMap contributors';
          results.append(credit);
        }
      } else throw new Error('Unrecognized generation response.');
    } catch (error) {
      if (isCurrent()) onStatus(current.signal.aborted ? 'Generation canceled.' : error.message);
    } finally {
      if (controller === current) { build.disabled = document.querySelector('#version')?.value === 'manual'; cancel.hidden = true; }
    }
  });
  cancel.addEventListener('click', () => controller?.abort());
  return { cancel() { controller?.abort(); results.replaceChildren(); } };
}
