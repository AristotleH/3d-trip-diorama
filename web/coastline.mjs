// OSM coastlines have land on the left in geographic coordinates. Projecting
// north to -Z reverses winding: water is on the positive-cross-product side.
// Decompose water into non-overlapping trapezoids instead of inventing a closed
// ocean outline. This preserves concave shores and islands using SurfaceDef.
const EPS = 1e-7;
const cross = (a,b) => a[0]*b[1]-a[1]*b[0];
const sub = (a,b) => [a[0]-b[0],a[1]-b[1]];
const key = p => p.map(v=>v.toFixed(6)).join(',');

function clip(a,b,h) {
  let lo=0,hi=1; const d=sub(b,a);
  for(let axis=0;axis<2;axis++) {
    if(Math.abs(d[axis])<EPS) {if(Math.abs(a[axis])>h)return null;continue;}
    let t0=(-h-a[axis])/d[axis],t1=(h-a[axis])/d[axis];
    if(t0>t1)[t0,t1]=[t1,t0];
    lo=Math.max(lo,t0);hi=Math.min(hi,t1);
    if(hi-lo<EPS)return null;
  }
  return [lo,hi].map(t=>a.map((v,i)=>Math.max(-h,Math.min(h,v+t*d[i]))));
}

// An empty scanline has no coast crossings. Cast a ray toward a known shore
// and use the first intersection's directed edge to classify its entire row.
function isWater(point, segments) {
  for(const fraction of [0.413,0.617,0.281]) {
    const [a,b]=segments[0],target=a.map((v,i)=>v+(b[i]-v)*fraction);
    const ray=sub(target,point);
    let nearest=Infinity,wet=false,vertex=false;
    for(const [c,d] of segments) {
      const edge=sub(d,c),denom=cross(ray,edge);
      if(Math.abs(denom)<EPS)continue;
      const delta=sub(c,point),t=cross(delta,edge)/denom,u=cross(delta,ray)/denom;
      if(t>EPS && u>=-EPS && u<=1+EPS && t<nearest) {
        nearest=t; wet=cross(edge,sub(point,c))>0;
        vertex=u<EPS || u>1-EPS;
      }
    }
    if(Number.isFinite(nearest) && !vertex)return wet;
  }
  throw new Error('Could not determine the water side of the coastline. Try a slightly different center.');
}

export function coastalWater(elements, project, halfSize=36) {
  const segments=[],seen=new Set(),nodes=new Map();
  const boundary=p=>Math.abs(Math.abs(p[0])-halfSize)<EPS || Math.abs(Math.abs(p[1])-halfSize)<EPS;
  for(const e of elements) {
    if(e.type!=='way' || e.tags?.natural!=='coastline')continue;
    const g=e.geometry;
    if(!g || !g.every(p=>Number.isFinite(p.lat)&&Number.isFinite(p.lon)))
      throw new Error('The coastline response has missing coordinates. Please retry.');
    for(let i=1;i<g.length;i++) {
      const segment=clip(project(g[i-1]),project(g[i]),halfSize);
      if(!segment)continue;
      const [a,b]=segment,ka=key(a),kb=key(b),id=`${ka}/${kb}`;
      if(ka===kb || seen.has(id))continue;
      seen.add(id);segments.push(segment);
      for(const [p,k,slot] of [[a,ka,1],[b,kb,0]]) {
        if(boundary(p))continue;
        const degree=nodes.get(k)||[0,0];degree[slot]++;nodes.set(k,degree);
      }
    }
  }
  if(!segments.length)return {surfaces:[],segments:0};
  if(segments.length>4000)throw new Error('The coastline is too detailed for this crop. Use a smaller radius.');
  if([...nodes.values()].some(([incoming,outgoing])=>incoming!==1 || outgoing!==1))
    throw new Error('The coastline is incomplete or branches inside this crop. Retry or adjust the center.');

  const levels=[...new Set([-halfSize,halfSize,...segments.flatMap(s=>s.map(p=>p[1]))])].sort((a,b)=>a-b);
  const surfaces=[];
  const xAt=(s,y)=>s[0][0]+(y-s[0][1])*(s[1][0]-s[0][0])/(s[1][1]-s[0][1]);
  const add=(left,right,y0,y1)=>{
    const at=(s,y)=>typeof s==='number'?s:Math.max(-halfSize,Math.min(halfSize,xAt(s,y)));
    const raw=[[at(left,y0),y0],[at(right,y0),y0],[at(right,y1),y1],[at(left,y1),y1]];
    const points=raw.filter((p,i)=>Math.hypot(...sub(p,raw[(i+3)%4]))>EPS);
    const area=points.reduce((sum,p,i)=>sum+cross(p,points[(i+1)%points.length]),0)/2;
    if(points.length>=3 && area>EPS)surfaces.push({points,color:[.25,.45,.55],kind:'water',source:'coastline'});
    if(surfaces.length>4000)throw new Error('Too many coastal water sections. Use a smaller radius.');
  };
  for(let i=1;i<levels.length;i++) {
    const y0=levels[i-1],y1=levels[i];if(y1-y0<EPS)continue;
    const mid=(y0+y1)/2;
    const crossings=segments.filter(([a,b])=>Math.min(a[1],b[1])<mid && Math.max(a[1],b[1])>mid)
      .sort((a,b)=>xAt(a,mid)-xAt(b,mid));
    if(!crossings.length) {if(isWater([0,mid],segments))add(-halfSize,halfSize,y0,y1);continue;}
    let wet=crossings[0][1][1]>crossings[0][0][1],left=-halfSize;
    for(const edge of crossings) {
      const waterBefore=edge[1][1]>edge[0][1];
      if(wet!==waterBefore)throw new Error('The coastline direction is inconsistent. Try a different crop.');
      if(wet)add(left,edge,y0,y1);
      left=edge;wet=!wet;
    }
    if(wet)add(left,halfSize,y0,y1);
  }
  return {surfaces,segments:segments.length};
}
