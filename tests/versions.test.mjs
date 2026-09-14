import {test} from 'node:test';
import assert from 'node:assert/strict';
import {selectedVersion, setupVersions} from '../web/versions.mjs';
test('manual is default; explicit links take precedence over saved choice',()=>{
  assert.equal(selectedVersion('',null),'manual');
  assert.equal(selectedVersion('','llm'),'llm');
  assert.equal(selectedVersion('?version=manual','llm'),'manual');
  assert.equal(selectedVersion('?version=llm',null),'llm');
  assert.equal(selectedVersion('?version=unknown','llm'),'manual');
});
test('manual works without a backend and AI remains disabled when unavailable',async()=>{
  const nodes={version:{value:'',addEventListener(k,f){this[k]=f;}},'llm-panel':{},'llm-availability':{},'prompt-build':{}};
  globalThis.document={querySelector:s=>nodes[s.slice(1)]};
  globalThis.location={search:'',href:'https://example.test/'};
  globalThis.localStorage={getItem:()=>null,setItem(){}};
  globalThis.history={replaceState(){}};
  let changed=0;
  await setupVersions({onChange:()=>changed++,fetchImpl:async()=>{throw new Error('no backend');}});
  assert.equal(nodes['llm-panel'].hidden,true);
  nodes.version.value='llm';nodes.version.change();
  assert.equal(nodes['llm-panel'].hidden,false);
  assert.equal(nodes['prompt-build'].disabled,true);
  assert.equal(changed,1);
});
