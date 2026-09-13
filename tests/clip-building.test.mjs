import {test} from 'node:test';
import assert from 'node:assert/strict';
import {clipBuilding} from '../web/clip-building.mjs';
import {convertOSM} from '../web/osm.mjs';
const area=p=>Math.abs(p.reduce((s,a,i)=>{const b=p[(i+1)%p.length];return s+a[0]*b[1]-a[1]*b[0];},0)/2);
const sum=parts=>parts.reduce((s,p)=>s+area(p),0);
const building=(points,id=1)=>({type:'way',id,tags:{building:'yes',height:'30'},geometry:[...points,points[0]].map(([x,z])=>({lat:-z/(111195*.072),lon:x/(111195*.072)}))});

test('all four edges and corners are clipped to exact bounds',()=>{
 for(let turn=0;turn<4;turn++) {
  const rotate=([x,y])=>{for(let i=0;i<turn;i++)[x,y]=[-y,x];return [x,y];};
  const result=clipBuilding([[-1,-1],[3,-1],[3,1],[-1,1]].map(rotate),2);
  assert.equal(sum(result),6);
  assert.ok(result.flat(2).every(x=>Math.abs(x)<=2));
 }
 assert.equal(sum(clipBuilding([[1,1],[3,1],[3,3],[1,3]],2)),1);
});
test('fully outside and edge-touching footprints have no volume',()=>{
 assert.deepEqual(clipBuilding([[3,0],[4,0],[4,1],[3,1]],2),[]);
 assert.deepEqual(clipBuilding([[2,0],[3,0],[3,1],[2,1]],2),[]);
 assert.equal(sum(clipBuilding([[-3,-3],[3,-3],[3,3],[-3,3]],2)),16);
});
test('a concave footprint can split without bridging empty space',()=>{
 const parts=clipBuilding([[-1,-3],[3,-3],[3,3],[-1,3],[-1,1],[2.5,1],[2.5,-1],[-1,-1]],2);
 assert.equal(parts.length,2);
 assert.equal(sum(parts),6);
 assert.ok(parts.every(p=>p.every(v=>v[1]>=1)||p.every(v=>v[1]<=-1)));
});
test('OSM clipping preserves height and source while producing schema footprints',()=>{
 const s=convertOSM({elements:[building([[30,-4],[42,-4],[42,4],[30,4]])]},{lat:0,lon:0,radius:500});
 assert.equal(s.buildings.length,1);
 const b=s.buildings[0];
 assert.ok(Math.abs(b.height-2.16)<1e-10);
 assert.equal(b.osm_id,'way/1');assert.equal(b.clipped,true);
 assert.ok(b.footprint.flat().every(v=>Math.abs(v)<=36));
 assert.ok(b.footprint.some(p=>p[0]===36));
 assert.ok(Math.abs(area(b.footprint)-48)<1e-8);
 assert.equal(s.metadata.stats.clipped_buildings,1);
 assert.equal(s.metadata.stats.skipped,0);
});
test('OSM retains both pieces of a cut concave building with the same height',()=>{
 const shape=[[-18,-54],[54,-54],[54,54],[-18,54],[-18,18],[45,18],[45,-18],[-18,-18]];
 const s=convertOSM({elements:[building(shape)]},{lat:0,lon:0,radius:500});
 assert.equal(s.buildings.length,2);
 assert.equal(s.buildings[0].height,s.buildings[1].height);
 assert.equal(s.buildings[0].osm_id,s.buildings[1].osm_id);
 assert.ok(Math.abs(sum(s.buildings.map(b=>b.footprint))-1944)<1e-6);
});
