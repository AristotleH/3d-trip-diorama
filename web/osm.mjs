// Pure OSM → diorama conversion. Coordinates use a local tangent-plane approximation.
import { coastalWater } from './coastline.mjs';
import { clipPolygon } from './clip-building.mjs?v=coverage-1';
export const MAX_VERTICES = 250000;
export const ENDPOINT = 'https://overpass-api.de/api/interpreter';
export function validateCenter(lat, lon, radius) {
  if (![lat, lon, radius].every(Number.isFinite) || Math.abs(lat) > 80 || Math.abs(lon) > 180 || radius < 100 || radius > 1000)
    throw new Error('Use latitude −80…80, longitude −180…180 and radius 100…1000 metres.');
  const dLat = radius / 111195, dLon = dLat / Math.cos(lat * Math.PI / 180);
  if (Math.abs(lon) + dLon >= 180) throw new Error('Areas crossing the date line are not supported.');
  return [lat - dLat, lon - dLon, lat + dLat, lon + dLon];
}
export function queryFor(lat, lon, radius) {
  const bbox = validateCenter(lat, lon, radius).join(',');
  return `[out:json][timeout:25][maxsize:16777216];(way[building][building!="no"](${bbox});way[highway](${bbox});way[natural=coastline](${bbox});way[natural=water](${bbox});way[waterway=riverbank](${bbox});way[leisure=park](${bbox});relation[type=multipolygon][building](${bbox});relation[type=multipolygon][natural=water](${bbox});relation[type=multipolygon][leisure=park](${bbox}););out geom;`;
}
export function metres(value) {
  if (typeof value !== 'string' && typeof value !== 'number') return null;
  const m = String(value).trim().match(/^(\d+(?:\.\d+)?)\s*(m|meters|metres|ft|feet|')?$/i);
  if (!m) return null;
  const n = Number(m[1]) * (/^(ft|feet|')$/i.test(m[2] || '') ? .3048 : 1);
  return n > 0 && n <= 1000 ? n : null;
}
export function buildingHeight(tags) {
  const explicit = metres(tags.height);
  if (explicit) return { metres: explicit, source: 'height' };
  const levels = Number(tags['building:levels']);
  if (levels > 0 && levels <= 200) return { metres: levels * 3, source: 'levels × 3m' };
  return { metres: ['house', 'detached', 'residential'].includes(tags.building) ? 9 : 12, source: 'default estimate' };
}
const area = p => p.reduce((a, v, i) => { const w = p[(i + 1) % p.length]; return a + v[0] * w[1] - w[0] * v[1]; }, 0) / 2;
const same = (a,b) => a && b && a.lat === b.lat && a.lon === b.lon;
// Preserve both outer rings and courtyard/island rings from relation members.
function ringsFor(e, role='outer') {
  if (e.type === 'way') return e.geometry ? [e.geometry] : [];
  const members=(e.members||[]).filter(m=>m.type==='way' && (m.role===role || (!m.role && role==='outer')));
  if(members.some(m=>!m.geometry?.length))return [];
  const parts = members.map(m=>m.geometry.slice());
  const rings = [];
  while (parts.length) {
    const ring = parts.shift();
    while (!same(ring[0], ring.at(-1))) {
      const i = parts.findIndex(p => same(ring.at(-1), p[0]) || same(ring.at(-1), p.at(-1)));
      if (i < 0) return [];
      const p = parts.splice(i,1)[0];
      if (!same(ring.at(-1), p[0])) p.reverse();
      ring.push(...p.slice(1));
    }
    rings.push(ring);
  }
  return rings;
}
function contains(ring, point) {
  let inside=false;
  for(let i=0,j=ring.length-1;i<ring.length;j=i++) {
    const a=ring[i],b=ring[j];
    if((a.lat>point.lat)!==(b.lat>point.lat) && point.lon<(b.lon-a.lon)*(point.lat-a.lat)/(b.lat-a.lat)+a.lon)inside=!inside;
  }
  return inside;
}
function clipSegment(a,b,h) {
  let low=0, high=1; const d=[b[0]-a[0],b[1]-a[1]];
  for (let i=0;i<2;i++) {
    if (Math.abs(d[i])<1e-10) { if(Math.abs(a[i])>h)return null; continue; }
    let t0=(-h-a[i])/d[i], t1=(h-a[i])/d[i];
    if(t0>t1)[t0,t1]=[t1,t0]; low=Math.max(low,t0); high=Math.min(high,t1);
    if(low>=high)return null;
  }
  return [[a[0]+low*d[0],a[1]+low*d[1]],[a[0]+high*d[0],a[1]+high*d[1]]];
}
export function convertOSM(data, { lat, lon, radius }) {
  validateCenter(lat,lon,radius);
  if (!Array.isArray(data.elements)) throw new Error('Expected Overpass JSON with an elements array.');
  if (data.remark) throw new Error(`OSM returned an incomplete response: ${data.remark}`);
  if (data.elements.length > 25000) throw new Error('Too many OSM features. Choose a smaller radius.');
  const scale=36/radius, k=111195, cos=Math.cos(lat*Math.PI/180);
  const project = p => [(p.lon-lon)*k*cos*scale, (lat-p.lat)*k*scale];
  const stats={buildings:0, roads:0, water:0, parks:0, estimated_heights:0, skipped:0, vertices:0};
  const scene={name:`OSM ${lat.toFixed(5)}, ${lon.toFixed(5)}`,terrain:{half_size:36,resolution:32,base_color:[.57,.62,.48]},slab:{depth:1.8},buildings:[],roads:[],water:[],trees:[],surfaces:[],material_scale:1/(scale*2.3),metadata:{source:'OpenStreetMap',attribution:'© OpenStreetMap contributors',license:'ODbL-1.0',license_url:'https://www.openstreetmap.org/copyright',center:{lat,lon},radius_m:radius,coverage:'Square extending radius metres north/south/east/west; buildings and coastal water clipped to the boundary; other boundary-crossing polygons omitted.',metres_per_unit:1/scale,elevation:'Flat ground; no elevation dataset',osm_timestamp:data.osm3s?.timestamp_osm_base || null,stats}};
  const usedMembers=new Set();
  const skip=reason=>{
    stats.skipped++;
    stats.omitted_by_reason ||= {};
    stats.omitted_by_reason[reason]=(stats.omitted_by_reason[reason]||0)+1;
  };
  const coast=coastalWater(data.elements,project);
  scene.surfaces.push(...coast.surfaces);
  stats.coastal_water=coast.surfaces.length>0;
  stats.coastline_segments=coast.segments;
  stats.vertices=coast.surfaces.reduce((n,s)=>n+s.points.length,0);
  scene.metadata.coastal_water=stats.coastal_water?'Generated from directed OSM coastlines':'No intersecting coastline in this response; open ocean cannot be inferred without a shore.';
  scene.metadata.coverage='Square extending radius metres north/south/east/west; buildings, parks and water clipped to its boundary, with courtyards and islands preserved.';
  const seen=new Set();
  const elements=[...data.elements].sort((a,b)=>{
    const relationOrder=Number(b.type==='relation')-Number(a.type==='relation');
    if(relationOrder)return relationOrder;
    const center=e=>e.center || e.geometry?.[0] || e.members?.[0]?.geometry?.[0] || {lat,lon};
    const dist=e=>Math.hypot(...project(center(e)));
    return dist(a)-dist(b);
  });
  for(const e of elements) {
    const key=`${e.type}/${e.id}`,t=e.tags||{};
    if(t.natural==='coastline' || seen.has(key) || usedMembers.has(key))continue;
    seen.add(key);
    if(stats.vertices>=MAX_VERTICES){skip('geometry_limit');continue;}
    if(t.highway && !t.building) {
      if(scene.roads.length>=5000){skip('road_limit');continue;}
      if(['proposed','construction'].includes(t.highway)||t.area==='yes'||t.tunnel==='yes'){skip('unsupported_road');continue;}
      const g=e.geometry||[];
      const width=(metres(t.width)||({motorway:18,trunk:14,primary:10,secondary:8,tertiary:7,residential:6,service:4,footway:1.5,path:1.5}[t.highway]||4))*scale;
      for(let i=1;i<g.length;i++) {
        if(scene.roads.length>=5000){skip('road_limit');break;}
        if(stats.vertices+4>MAX_VERTICES){skip('geometry_limit');break;}
        if(![g[i-1].lat,g[i-1].lon,g[i].lat,g[i].lon].every(Number.isFinite))continue;
        const points=clipSegment(project(g[i-1]),project(g[i]),36-width/2);
        if(points){scene.roads.push({points,width});stats.roads++;stats.vertices+=4;}
      }
      continue;
    }
    const isBuilding=!!t.building && t.building!=='no';
    const isWater=t.natural==='water'||t.waterway==='riverbank';
    if(!isBuilding&&!isWater&&t.leisure!=='park')continue;
    const rings=ringsFor(e),inner=e.type==='relation'?ringsFor(e,'inner'):[];
    const valid=ring=>same(ring[0],ring.at(-1))&&ring.length>=4&&ring.length<=4097&&ring.every(p=>Number.isFinite(p.lat)&&Number.isFinite(p.lon));
    if(!rings.length || ![...rings,...inner].every(valid) ||
        (e.members?.some(m=>m.role==='inner')&&!inner.length)){
      skip('invalid_geometry');continue;
    }
    const groups=rings.map(outer=>({outer,inner:inner.filter(hole=>contains(outer,hole[0]))}));
    if(groups.reduce((n,g)=>n+g.inner.length,0)!==inner.length){skip('invalid_geometry');continue;}
    const before=stats.vertices;
    for(const group of groups) {
      const clean=ring=>ring.slice(0,-1).map(project).filter((p,i,a)=>i===0||Math.hypot(p[0]-a[i-1][0],p[1]-a[i-1][1])>1e-5);
      const original=clean(group.outer),originalHoles=group.inner.map(clean);
      if(original.length<3||Math.abs(area(original))<.0001||originalHoles.some(h=>h.length<3||Math.abs(area(h))<.0001)){skip('invalid_geometry');continue;}
      const crossesEdge=original.some(p=>Math.abs(p[0])>36||Math.abs(p[1])>36);
      let polygons=[{outer:original,holes:[]}];
      if(crossesEdge||originalHoles.length) {
        try{polygons=clipPolygon(original,originalHoles,36);}
        catch{skip('invalid_geometry');continue;}
      }
      if(!polygons.length){skip('outside_crop');continue;}
      for(const {outer:polygon,holes} of polygons) {
        if(polygon.length<3||Math.abs(area(polygon))<.0001){skip('invalid_geometry');continue;}
        if(isBuilding&&stats.buildings>=2500){skip('building_limit');continue;}
        if(!isBuilding&&stats.water+stats.parks>=1000){skip('surface_limit');continue;}
        if(area(polygon)<0)polygon.reverse();
        for(const hole of holes)if(area(hole)>0)hole.reverse();
        const vertexCost=(polygon.length+holes.reduce((n,h)=>n+h.length,0))*(isBuilding?10:1);
        if(stats.vertices+vertexCost>MAX_VERTICES){skip('geometry_limit');continue;}
        stats.vertices+=vertexCost;
        const holeField=holes.length?{holes}:{};
        if(isBuilding) {
          const h=buildingHeight(t);if(h.source!=='height')stats.estimated_heights++;
          const xs=polygon.map(p=>p[0]),zs=polygon.map(p=>p[1]);
          const minX=Math.min(...xs),maxX=Math.max(...xs),minZ=Math.min(...zs),maxZ=Math.max(...zs);
          scene.buildings.push({cx:(minX+maxX)/2,cz:(minZ+maxZ)/2,width:maxX-minX,depth:maxZ-minZ,height:h.metres*scale,footprint:polygon,...holeField,color:t.building==='industrial'?[.63,.67,.7]:[.84,.77,.65],osm_id:key,height_source:h.source,...(crossesEdge?{clipped:true}:{})});
          stats.buildings++;
          if(holes.length)stats.courtyard_buildings=(stats.courtyard_buildings||0)+1;
          if(crossesEdge)stats.clipped_buildings=(stats.clipped_buildings||0)+1;
        } else {
          scene.surfaces.push({points:polygon,...holeField,color:isWater?[.25,.45,.55]:[.37,.57,.31],kind:isWater?'water':'park'});
          stats[isWater?'water':'parks']++;
        }
      }
    }
    // Suppress duplicates only after the parent actually produced geometry.
    if(e.type==='relation'&&stats.vertices>before)
      for(const member of e.members||[])if(member.type==='way')usedMembers.add(`way/${member.ref}`);
  }
  if(!stats.buildings&&!stats.roads&&!stats.water&&!stats.parks&&!stats.coastal_water)throw new Error('No supported features found. Try a different center or larger radius.');
  return scene;
}
const memoryCache=new Map();
export async function fetchOSM(lat,lon,radius,signal) {
  const query=queryFor(lat,lon,radius),key=`${lat},${lon},${radius}`;
  const cached=memoryCache.get(key); if(cached && Date.now()-cached.at<3600000)return cached.data;
  const response=await fetch(ENDPOINT,{method:'POST',headers:{'Content-Type':'application/x-www-form-urlencoded'},body:new URLSearchParams({data:query}),signal});
  if(!response.ok)throw new Error(response.status===429?'OSM service is busy. Wait a minute or use a smaller radius.':`OSM service returned ${response.status}. Try again later.`);
  const reader=response.body.getReader();const chunks=[];let size=0;
  while(true){const {done,value}=await reader.read();if(done)break;size+=value.length;if(size>16*1024*1024){await reader.cancel();throw new Error('OSM response too large. Choose a smaller radius.');}chunks.push(value);}
  const bytes=new Uint8Array(size);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}
  let data;
  try { data=JSON.parse(new TextDecoder().decode(bytes)); } catch { throw new Error('OSM returned an incomplete response. Try a smaller radius or try again later.'); }
  if(data.remark)throw new Error('OSM query timed out or returned partial data. Choose a smaller radius.');
  if(memoryCache.size>=3)memoryCache.delete(memoryCache.keys().next().value);
  memoryCache.set(key,{at:Date.now(),data});return data;
}
