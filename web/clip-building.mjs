import clipping from './vendor/polygon-clipping.mjs';

// Return every disconnected part of the intersection, not an artificial bridge
// across a concave building. Results remain ordinary schema footprints.
export function clipBuilding(footprint, halfSize) {
  return clipPolygon(footprint,[],halfSize).map(p=>p.outer);
}

export function clipPolygon(footprint, holes, halfSize) {
  const h=halfSize;
  const square=[[[-h,-h],[h,-h],[h,h],[-h,h],[-h,-h]]];
  return clipping.intersection([footprint,...holes],square).map(polygon=>{
    const rings=polygon.map(ring=>ring.slice(0,-1).map(p=>p.map(v=>Math.max(-h,Math.min(h,v)))));
    return {outer:rings[0],holes:rings.slice(1)};
  });
}
