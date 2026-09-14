export function selectedVersion(search, saved) {
  const requested = new URLSearchParams(search).get('version');
  return (requested ?? saved) === 'llm' ? 'llm' : 'manual';
}

export async function setupVersions({ onChange, fetchImpl = fetch }) {
  const picker = document.querySelector('#version');
  const panel = document.querySelector('#llm-panel');
  const notice = document.querySelector('#llm-availability');
  const build = document.querySelector('#prompt-build');
  if (!picker) return;
  let saved;
  try { saved = localStorage.getItem('diorama-version'); } catch {}
  picker.value = selectedVersion(location.search, saved);
  let available = false;
  const apply = (changed = false) => {
    const llm = picker.value === 'llm';
    panel.hidden = !llm;
    build.disabled = !llm || !available;
    if (changed) {
      onChange();
      try { localStorage.setItem('diorama-version', picker.value); } catch {}
      const url = new URL(location.href);
      url.searchParams.set('version', picker.value);
      history.replaceState(null, '', url);
    }
  };
  picker.addEventListener('change', () => apply(true));
  apply();
  try {
    const response = await fetchImpl('/api/config');
    if (!response.ok) throw new Error('Unavailable');
    const config = await response.json();
    available = config.llmEnabled === true && config.llmConfigured === true;
    notice.textContent = available ? '' : 'AI generation is not configured for this site yet. The manual version is fully available.';
  } catch { notice.textContent = 'This host supports the manual version. AI generation needs the application backend.'; }
  apply();
}
