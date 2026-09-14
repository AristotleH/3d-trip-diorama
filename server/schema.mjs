// One schema drives the model contract and local validation. Keep this subset
// bounded: generated scenes do not need arbitrary polygons or their topology.
const number = (minimum, maximum) => ({ type: 'number', minimum, maximum });
const string = maxLength => ({ type: 'string', minLength: 1, maxLength });
const array = (items, maxItems, minItems = 0) => ({ type: 'array', items, minItems, maxItems });
const object = properties => ({ type: 'object', properties, required: Object.keys(properties), additionalProperties: false });
const color = array(number(0, 1), 3, 3);
const position = number(-36, 36);
export const sceneSchema = object({
  name: string(100),
  terrain: object({
    half_size: number(8, 36),
    resolution: { type: 'integer', minimum: 8, maximum: 80 },
    base_color: color,
    control_points: array(object({ x: position, z: position, amplitude: number(-4, 6), sigma: number(1, 20) }), 12),
    color_zones: array(object({ max_height: number(-20, 30), color }), 8),
  }),
  buildings: array(object({ cx: position, cz: position, width: number(.2, 12), depth: number(.2, 12), height: number(.2, 30), color, shape: { type: 'string', enum: ['rect', 'l_shape', 't_shape'] } }), 120),
  trees: array(object({ x: position, z: position, canopy_radius: number(.2, 3), canopy_height: number(.3, 8), leaf_color: color }), 150),
  roads: array(object({ points: array(array(position, 2, 2), 16, 2), width: number(.2, 3) }), 30),
  water: array(object({ cx: position, cz: position, radius_x: number(.3, 15), radius_z: number(.3, 15) }), 8),
});
export const placeSchema = object({ query: string(250), radius_m: number(100, 1000) });

export function validate(value, schema, path = 'result') {
  if (schema.type === 'object') {
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(`${path} must be an object`);
    for (const key of Object.keys(value)) if (!Object.hasOwn(schema.properties, key)) throw new Error(`${path}.${key} is not supported`);
    for (const key of schema.required) validate(value[key], schema.properties[key], `${path}.${key}`);
  } else if (schema.type === 'array') {
    if (!Array.isArray(value) || value.length < schema.minItems || value.length > schema.maxItems) throw new Error(`${path} needs ${schema.minItems}–${schema.maxItems} items`);
    value.forEach((item, i) => validate(item, schema.items, `${path}[${i}]`));
  } else if (schema.type === 'string') {
    if (typeof value !== 'string' || (schema.minLength && value.trim().length < schema.minLength) || value.length > (schema.maxLength ?? Infinity) || (schema.enum && !schema.enum.includes(value))) throw new Error(`${path} is not a supported string`);
  } else if (!Number.isFinite(value) || value < schema.minimum || value > schema.maximum || (schema.type === 'integer' && !Number.isInteger(value))) {
    throw new Error(`${path} must be ${schema.minimum}–${schema.maximum}${schema.type === 'integer' ? ' (integer)' : ''}`);
  }
  return value;
}

export function validateScene(scene) {
  validate(scene, sceneSchema);
  const h = scene.terrain.half_size;
  const within = (x, z, rx = 0, rz = 0) => {
    if (Math.abs(x) + rx > h || Math.abs(z) + rz > h) throw new Error('All features must fit within terrain.half_size');
  };
  scene.buildings.forEach(b => within(b.cx, b.cz, b.width / 2, b.depth / 2));
  scene.trees.forEach(t => within(t.x, t.z, t.canopy_radius, t.canopy_radius));
  scene.water.forEach(w => {
    within(w.cx, w.cz, w.radius_x, w.radius_z);
    const height = scene.terrain.control_points.reduce((sum, p) => sum + p.amplitude * Math.exp(-((w.cx - p.x) ** 2 + (w.cz - p.z) ** 2) / (2 * p.sigma ** 2)), 0);
    if (height >= -.2) throw new Error('Water needs a terrain depression below -0.2 at its center; water renders at -0.15');
  });
  scene.roads.forEach(r => r.points.forEach(([x, z], i) => {
    within(x, z, r.width / 2, r.width / 2);
    if (i && Math.hypot(x - r.points[i - 1][0], z - r.points[i - 1][1]) < .01) throw new Error('Road points must be distinct');
  }));
  const zones = scene.terrain.color_zones;
  if (zones.some((z, i) => i && z.max_height <= zones[i - 1].max_height)) throw new Error('Color zones must have strictly increasing heights');
  if (scene.terrain.control_points.reduce((sum, p) => sum + Math.abs(p.amplitude), 0) > 15) throw new Error('Total terrain amplitude must not exceed 15');
  return scene;
}
