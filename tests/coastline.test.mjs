import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {coastalWater} from '../web/coastline.mjs';
import {convertOSM,queryFor} from '../web/osm.mjs';
const way=(points,id=1)=>({type:'way',id,tags:{natural:'coastline'},geometry:points.map(([x,z])=>({lon:x,lat:z}))});
const project=p=>[p.lon,p.lat];
const area=p=>p.reduce((sum,a,i)=>{const b=p[(i+1)%p.length];return sum+a[0]*b[1]-b[0]*a[1];},0)/2;
function check(elements,expected) {
 const result=coastalWater(elements,project,2);
 assert.ok(Math.abs(result.surfaces.reduce((sum,s)=>sum+area(s.points),0)-expected)<1e-6);
 for(const s of result.surfaces) {
  assert.ok(area(s.points)>0);
  assert.ok(s.points.length===3 || s.points.length===4);
  assert.ok(s.points.flat().every(n=>Number.isFinite(n)&&Math.abs(n)<=2));
 }
 return result;
}
test('straight and horizontal coasts fill only their water side',()=>{
 const east=check([way([[0,3],[0,-3]])],8);
 assert.ok(east.surfaces.every(s=>s.points.every(p=>p[0]>=0)));
 const south=check([way([[-3,0],[3,0]])],8);
 assert.ok(south.surfaces.every(s=>s.points.every(p=>p[1]>=0)));
 check([way([[0,-3],[0,3]])],8);
});
test('split unordered ways and duplicate features preserve a concave bay',()=>{
 const a=way([[0,3],[0,1],[-1,1]],1),b=way([[-1,1],[-1,-1],[0,-1],[0,-3]],2);
 check([b,a,a],10);
});
test('islands stay dry, including beside mainland and across empty scanlines',()=>{
 check([way([[-1,-1],[-1,1],[1,1],[1,-1],[-1,-1]])],12);
 check([way([[0,3],[0,-3]],1),way([[.5,-.5],[.5,.5],[1.5,.5],[1.5,-.5],[.5,-.5]],2)],7);
});
test('sloped shores produce triangles without leaking outside the crop',()=>{
 check([way([[-3,3],[3,-3]])],8);
});
test('no coast does not invent water; broken coast fails explicitly',()=>{
 assert.equal(coastalWater([],project,2).surfaces.length,0);
 assert.throws(()=>coastalWater([way([[0,0],[0,3]])],project,2),/incomplete/);
 assert.throws(()=>coastalWater([way([[0,3],[0,0]],1),way([[0,-3],[0,0]],2)],project,2),/incomplete/);
});
test('OSM requests coastline and coastal-only imports work with existing schema',()=>{
 assert.match(queryFor(0,0,500),/way\[natural=coastline\]/);
 // Northward in geographic coordinates: sea to the east.
 const coast=way([[0,-.01],[0,.01]]);
 const scene=convertOSM({elements:[coast]},{lat:0,lon:0,radius:500});
 assert.equal(scene.metadata.stats.coastal_water,true);
 assert.ok(Math.abs(scene.surfaces.reduce((sum,s)=>sum+area(s.points),0)-2592)<1e-5);
 assert.ok(scene.surfaces.every(s=>s.points.every(p=>p[0]>=0)));
});

test('real Mission Bay coast includes the bay and creek while piers remain dry',()=>{
 const data=JSON.parse(readFileSync(new URL('./fixtures/mission-bay-coast.json',import.meta.url)));
 const scene=convertOSM(data,{lat:37.77453,lon:-122.389813,radius:1000});
 const fixture=JSON.parse(readFileSync(new URL('./fixtures/mission-bay-water-scene.json',import.meta.url)));
 assert.deepEqual(scene,fixture,'keep Rust triangulation fixture synchronized');
 const total=scene.surfaces.reduce((sum,s)=>sum+area(s.points),0);
 assert.ok(total>1800 && total<1900,'roughly 36% of this crop is coastal water');
 const contains=(p,poly)=>{
   let inside=false;
   for(let i=0,j=poly.length-1;i<poly.length;j=i++) {
     const a=poly[i],b=poly[j];
     if((a[1]>p[1])!==(b[1]>p[1]) && p[0]<(b[0]-a[0])*(p[1]-a[1])/(b[1]-a[1])+a[0])inside=!inside;
   }
   return inside;
 };
 const wet=p=>scene.surfaces.some(s=>contains(p,s.points));
 assert.equal(wet([30,1]),true,'open bay');
 assert.equal(wet([-10,0.5]),true,'Mission Creek');
 assert.equal(wet([-25,-20]),false,'mainland');
 assert.equal(wet([20,2]),false,'pier');
});
