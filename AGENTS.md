# Maintaining the site

Maintain both site versions from one shared codebase: Without AI (gallery and
manual OSM import) and With AI (the same features plus plaintext generation).
Keep `?version=manual` and `?version=llm` links working. Default to manual mode.
Never require an LLM key for the manual version. Turning AI off must cancel
pending generation and prevent stale output from replacing the scene.
`LLM_ENABLED=false` must also reject generation at the backend, not just hide UI.

Run the JavaScript suite, including version tests, for changes to either flow.
Use the existing ChatGPT Site identified by `.openai/hosting.json`; preserve its
audience. Build both modes with `node tools/build-sites.mjs`. Configure secrets
through Sites runtime settings, never in client assets or hosting.json.
