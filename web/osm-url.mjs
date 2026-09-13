import { validateCenter } from './osm.mjs?v=coverage-1';

export function osmHash({ lat, lon, radius }) {
  validateCenter(lat, lon, radius);
  return `#osm?${new URLSearchParams({ lat, lon, radius })}`;
}

export function parseOSMHash(hash) {
  if (!hash.startsWith('#osm?')) return null;
  const query = new URLSearchParams(hash.slice(5));
  const values = ['lat', 'lon', 'radius'].map(key => {
    const value = query.get(key);
    if (query.getAll(key).length !== 1 || !value?.trim())
      throw new Error('The OSM link needs one latitude, longitude and radius.');
    return Number(value);
  });
  validateCenter(...values);
  return { lat: values[0], lon: values[1], radius: values[2] };
}
