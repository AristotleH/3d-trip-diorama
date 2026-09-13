import { test } from 'node:test';
import assert from 'node:assert/strict';
import { convertOSM, metres, buildingHeight, queryFor, MAX_VERTICES } from '../web/osm.mjs';
const center={lat:0,lon:0,radius:500};
const ring=[{lat:0,lon:0},{lat:0,lon:.0001},{lat:.0001,lon:.0001},{lat:.0001,lon:0},{lat:0,lon:0}];
const building={type:'way',id:1,tags:{building:'yes',height:'30 ft'},geometry:ring};
test('height units, levels and missing heights are explicit',()=>{
 assert.equal(metres('30 ft'),9.144);assert.equal(metres('12;15'),null);
 assert.equal(buildingHeight({'building:levels':'4'}).metres,12);
 assert.equal(buildingHeight({}).source,'default estimate');
});
test('real footprint uses east +X and north −Z; scale and attribution survive export',()=>{
 const s=convertOSM({elements:[building,building]},center);
 assert.equal(s.buildings.length,1);assert.ok(Math.abs(s.buildings[0].height-9.144*36/500)<1e-12);
 assert.equal(s.buildings[0].footprint.length,4);assert.ok(s.buildings[0].cz<0);assert.ok(s.buildings[0].cx>0);
 assert.equal(s.metadata.license,'ODbL-1.0');assert.equal(s.metadata.stats.estimated_heights,0);
});
test('reassembles multipolygon outer ways and suppresses duplicates',()=>{
 const relation={type:'relation',id:9,tags:{building:'yes'},members:[{type:'way',ref:1,role:'outer',geometry:ring.slice(0,3)},{type:'way',ref:2,role:'outer',geometry:ring.slice(2)}]};
 const s=convertOSM({elements:[relation,building]},center);assert.equal(s.buildings.length,1);assert.equal(s.buildings[0].osm_id,'relation/9');
});
test('coincident invalid courtyard rings are omitted instead of filled',()=>{
 const relation={type:'relation',id:9,tags:{building:'yes'},members:[{type:'way',ref:2,role:'outer',geometry:ring},{type:'way',ref:3,role:'inner',geometry:ring}]};
 const s=convertOSM({elements:[building,relation]},center);assert.equal(s.buildings.length,1);assert.equal(s.metadata.stats.skipped,1);
});
test('roads crossing the complete area are clipped to its edges',()=>{
 const s=convertOSM({elements:[{type:'way',id:5,tags:{highway:'residential'},geometry:[{lat:0,lon:-.1},{lat:0,lon:.1}]}]},center);
 assert.equal(s.roads.length,1);assert.ok(s.roads[0].points.every(p=>Math.abs(p[0])<=36));
});
test('parks and water become polygon surfaces',()=>{
 const s=convertOSM({elements:[{...building,id:2,tags:{natural:'water'}},{...building,id:3,tags:{leisure:'park'}}]},center);
 assert.deepEqual(s.surfaces.map(x=>x.kind),['water','park']);
});
test('hitting the road limit does not discard subsequent buildings or parks',()=>{
 const geometry=Array.from({length:5002},(_,i)=>({lat:0,lon:i%2?.00001:0}));
 const s=convertOSM({elements:[{type:'way',id:100,tags:{highway:'residential'},geometry},building,{...building,id:101,tags:{leisure:'park'}}]},center);
 assert.equal(s.roads.length,5000);
 assert.equal(s.buildings.length,1);
 assert.equal(s.surfaces.length,1);
 assert.equal(s.metadata.stats.skipped,1);
});
test('one long road cannot overrun the shared geometry budget',()=>{
 const outline=Array.from({length:10},(_,i)=>({lat:.0001*(1-Math.cos(i*Math.PI/5)),lon:.0001*Math.sin(i*Math.PI/5)}));
 outline.push(outline[0]);
 const buildings=Array.from({length:2400},(_,i)=>({...building,id:i+1,geometry:outline}));
 const geometry=Array.from({length:5002},(_,i)=>({lat:.0001,lon:i%2?.0002:.0001}));
 const s=convertOSM({elements:[...buildings,{type:'way',id:10000,tags:{highway:'residential'},geometry}]},center);
 assert.equal(s.buildings.length,2400);
 assert.equal(s.metadata.stats.vertices,MAX_VERTICES);
 assert.equal(s.roads.length,2500);
 assert.equal(s.metadata.stats.omitted_by_reason.geometry_limit,1);
});
test('invalid centers, incomplete responses, empty results and oversized polygons fail safely',()=>{
 assert.throws(()=>queryFor(90,0,500));assert.throws(()=>queryFor(0,0,1001));assert.throws(()=>queryFor(0,179.999,1000));
 assert.throws(()=>convertOSM({remark:'timeout',elements:[building]},center));
 assert.throws(()=>convertOSM({elements:[]},center));
 const s=convertOSM({elements:[building,{...building,id:2,geometry:Array(600).fill(ring[0])}]},center);assert.equal(s.metadata.stats.skipped,1);
});
