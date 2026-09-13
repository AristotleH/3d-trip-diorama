import { test } from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
const source=readFileSync(new URL('../web/osm-ui.mjs',import.meta.url),'utf8').replace(/^import .*$/gm,'').replace('export function','function');
function ui(fetchOSM,isCurrent=()=>true){
 const elements=Object.fromEntries(['osm-form','osm-build','osm-cancel','osm-close','osm-details'].map(id=>[id,{addEventListener(e,fn){this[e]=fn;}}]));
 elements['osm-details'].open=true;
 elements['osm-form'].elements={latitude:{value:'0'},longitude:{value:'0'},radius:{value:'500'}};
 const scenes=[],errors=[];
 const context=vm.createContext({fetchOSM,convertOSM:data=>data,validateCenter(){},AbortController,setTimeout,clearTimeout,document:{querySelector:id=>elements[id.slice(1)]}});
 vm.runInContext(source,context);const api=context.setupOSM({onStart:()=>isCurrent,onScene:s=>scenes.push(s),onError:e=>errors.push(e)});
 return {elements,scenes,errors,load:api.load,submit:()=>elements['osm-form'].submit({preventDefault(){}})};
}
test('loading link parameters fills the form and uses the normal import path',async()=>{
 const calls=[];const g=ui(async(...args)=>{calls.push(args);return {name:'Mission Bay'};});
 await g.load({lat:37.77453,lon:-122.389813,radius:1000});
 assert.deepEqual(calls[0].slice(0,3),[37.77453,-122.389813,1000]);
 assert.equal(g.elements['osm-form'].elements.longitude.value,'-122.389813');
 assert.equal(g.scenes[0].name,'Mission Bay');
});
test('mobile sheet closes without changing the current scene',()=>{
 const g=ui(async()=>({}));g.elements['osm-close'].click();assert.equal(g.elements['osm-details'].open,false);assert.equal(g.scenes.length,0);
});
test('canceling a request never installs a scene and re-enables import',async()=>{
 const g=ui((a,b,c,signal)=>new Promise((resolve,reject)=>signal.addEventListener('abort',()=>reject(new Error('abort')))));
 const pending=g.submit();g.elements['osm-cancel'].click();await pending;
 assert.equal(g.scenes.length,0);assert.equal(g.errors[0],'Import canceled.');assert.equal(g.elements['osm-build'].disabled,false);
});
test('a gallery change while fetching suppresses the stale import',async()=>{
 let resolve;const g=ui(()=>new Promise(r=>resolve=r),()=>false);
 const pending=g.submit();resolve({name:'old'});await pending;
 assert.equal(g.scenes.length,0);assert.equal(g.elements['osm-build'].disabled,false);
});
