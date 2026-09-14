# Plaintext verification — 2026-09-13

The JavaScript suite has 61 passing tests, including HTTP integration tests using ephemeral localhost ports. New coverage exercises concurrent requests and slot recovery, malformed JSON, unsupported HTTP methods, oversized bodies/responses, client-disconnect cancellation, geocoder errors/empty results, water geometry, and invalid coordinate strings. Empty coordinate strings previously became (0, 0); this was reproduced with a failing test, fixed, and retested.

Browser layout measurements:

| Viewport | Document width | Canvas height | Result |
|---|---:|---:|---|
| 320 × 568 | 320 | 120 | No horizontal overflow; vertical scrolling |
| 375 × 667 | 375 | approximately 179 | Form, canvas and footer visible |
| 667 × 375 | 667 | 120 | No horizontal overflow; vertical scrolling |
| 768 × 1024 | 768 | 615 | No horizontal overflow |

At 320 × 568, the OSM bottom sheet opened and closed, its inputs and build button remained usable, and form submission displayed a disconnected-backend error while keeping Create enabled and the current scene intact. These are desktop-browser viewport tests, not physical touchscreen or mobile-keyboard tests. Temporary viewport overrides were reset.

Earlier browser verification used controlled LLM/geocoder responses: a generated scene rendered and was downloadable, then choosing a Tokyo Station candidate invoked live Overpass and rendered 458 building pieces and 4,560 road segments. The full wasm-pack build passed. These checks do not establish live LLM quality or geocoder availability.

The fixed suite in `tests/evals/prompts.json` covers scene composition, lake geometry, terrain, conflicting instructions, named places, radius/width interpretation, ambiguity and unsupported coverage. `tools/evaluate-llm.mjs` was run in dry mode and syntax-checked. Live mode has not been run: provider credentials/model and a chosen geocoder are still unconfigured. Live model evaluation is explicitly opt-in and saves outputs for manual visual review.
