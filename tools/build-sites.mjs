import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
const root = process.cwd();
const hosting = JSON.parse(await readFile(path.join(root, '.openai/hosting.json')));
if (!hosting.project_id || hosting.static) throw new Error('A server-backed Sites hosting.json is required');
const assets = {};
const types = {'.html':'text/html','.js':'text/javascript','.mjs':'text/javascript','.wasm':'application/wasm','.json':'application/json'};
async function visit(directory, prefix = '') {
  for (const entry of await readdir(directory, {withFileTypes:true})) {
    if (entry.name.startsWith('.')) continue;
    const relative = `${prefix}/${entry.name}`;
    const filename = path.join(directory,entry.name);
    if(entry.isDirectory()) await visit(filename,relative);
    else if(types[path.extname(filename)]) assets[relative]={type:types[path.extname(filename)],body:(await readFile(filename)).toString('base64')};
  }
}
await visit(path.join(root,'web'));
if(!assets['/pkg/diorama_app_bg.wasm'] || !assets['/index.html']) throw new Error('Build the WASM bundle first');
const modules=[];
for(const name of ['schema','llm','geocode','sites-worker']) {
  let source=await readFile(path.join(root,`server/${name}.mjs`),'utf8');
  source=source.replace(/^import .*;\n/gm,'').replace(/^export (?=(async )?function|const )/gm,'');
  modules.push(source);
}
await mkdir(path.join(root,'dist/server'),{recursive:true});
await mkdir(path.join(root,'dist/.openai'),{recursive:true});
await writeFile(path.join(root,'dist/server/index.js'),`const SITE_ASSETS=${JSON.stringify(assets)};\n${modules.join('\n')}`);
await writeFile(path.join(root,'dist/.openai/hosting.json'),JSON.stringify(hosting));
console.log(`Built both site versions: ${Object.keys(assets).length} assets and shared generation backend.`);
