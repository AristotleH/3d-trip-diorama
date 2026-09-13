import {test} from 'node:test';
import assert from 'node:assert/strict';
import {convertOSM} from '../web/osm.mjs';
const center={lat:0,lon:0,radius:500};
const ring=points=>[...points,points[0]].map(([x,z])=>({lat:-z/(111195*.072),lon:x/(111195*.072)}));
const outer=ring([[-10,-10],[10,-10],[10,10],[-10,10]]);
const hole=ring([[-3,-3],[3,-3],[3,3],[-3,3]]);
const relation=(tags={building:'yes'})=>({type:'relation',id:1,tags,members:[{type:'way',ref:10,role:'outer',geometry:outer},{type:'way',ref:11,role:'inner',geometry:hole}]});

test('courtyard relation renders once with the hole preserved',()=>{
 const s=convertOSM({elements:[{type:'way',id:10,tags:{building:'yes'},geometry:outer},relation()]},center);
 assert.equal(s.buildings.length,1);
 assert.equal(s.buildings[0].holes.length,1);
 assert.equal(s.buildings[0].osm_id,'relation/1');
 assert.equal(s.metadata.stats.courtyard_buildings,1);
 assert.equal(s.metadata.stats.skipped,0);
});
test('failed parent does not hide usable member buildings',()=>{
 const bad=relation();bad.members[0].geometry=outer.slice(0,2);
 const s=convertOSM({elements:[bad,{type:'way',id:10,tags:{building:'yes'},geometry:outer}]},center);
 assert.equal(s.buildings.length,1);
 assert.equal(s.buildings[0].osm_id,'way/10');
 assert.equal(s.metadata.stats.omitted_by_reason.invalid_geometry,1);
});
test('parks and inland water preserve islands and clip at the boundary',()=>{
 for(const tags of [{leisure:'park'},{natural:'water'}]) {
   const r=relation(tags);
   r.members[0].geometry=ring([[-50,-50],[50,-50],[50,50],[-50,50]]);
   const s=convertOSM({elements:[r]},center);
   assert.equal(s.surfaces.length,1);assert.equal(s.surfaces[0].holes.length,1);
   assert.ok(s.surfaces[0].points.flat().every(n=>Math.abs(n)<=36));
 }
});
test('courtyard cut by the edge becomes an open notch',()=>{
 const r=relation();
 r.members[0].geometry=ring([[20,-10],[50,-10],[50,10],[20,10]]);
 r.members[1].geometry=ring([[30,-3],[45,-3],[45,3],[30,3]]);
 const s=convertOSM({elements:[r]},center);
 assert.equal(s.buildings.length,1);
 assert.equal(s.buildings[0].holes,undefined);
 assert.equal(s.buildings[0].footprint.length,8);
});
