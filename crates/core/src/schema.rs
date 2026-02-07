use serde::Deserialize;

/// Top-level diorama scene definition.
#[derive(Debug, Deserialize)]
pub struct DioramaScene {
    #[serde(default)]
    pub name: String,
    pub terrain: TerrainDef,
    #[serde(default)]
    pub slab: SlabDef,
    #[serde(default)]
    pub water: Vec<WaterDef>,
    #[serde(default)]
    pub roads: Vec<RoadDef>,
    #[serde(default)]
    pub buildings: Vec<BuildingDef>,
    #[serde(default)]
    pub trees: Vec<TreeDef>,
}

/// Terrain surface defined by Gaussian RBF control points.
#[derive(Debug, Deserialize)]
pub struct TerrainDef {
    #[serde(default = "default_half_size")]
    pub half_size: f32,
    #[serde(default = "default_resolution")]
    pub resolution: u32,
    #[serde(default)]
    pub control_points: Vec<TerrainControlPoint>,
    #[serde(default)]
    pub color_zones: Vec<ColorZone>,
    #[serde(default = "default_base_color")]
    pub base_color: [f32; 3],
}

fn default_half_size() -> f32 { 18.0 }
fn default_resolution() -> u32 { 80 }
fn default_base_color() -> [f32; 3] { [0.38, 0.55, 0.28] }

/// A single Gaussian RBF control point for terrain height.
#[derive(Debug, Deserialize)]
pub struct TerrainControlPoint {
    pub x: f32,
    pub z: f32,
    /// Peak height contribution at this point.
    pub amplitude: f32,
    /// Spread of the Gaussian (standard deviation).
    #[serde(default = "default_sigma")]
    pub sigma: f32,
}

fn default_sigma() -> f32 { 5.0 }

/// Height-based color zone for terrain. Zones are evaluated in order;
/// the first zone whose `max_height` is >= the terrain height is used.
#[derive(Debug, Deserialize)]
pub struct ColorZone {
    pub max_height: f32,
    pub color: [f32; 3],
}

/// Slab (cross-section base) definition.
#[derive(Debug, Deserialize)]
pub struct SlabDef {
    #[serde(default = "default_slab_depth")]
    pub depth: f32,
    #[serde(default = "default_slab_layers")]
    pub layers: Vec<SlabLayer>,
}

fn default_slab_depth() -> f32 { 5.0 }

fn default_slab_layers() -> Vec<SlabLayer> {
    vec![
        SlabLayer { depth_frac: 0.0,  color: [0.55, 0.42, 0.28] },
        SlabLayer { depth_frac: 0.15, color: [0.55, 0.42, 0.28] },
        SlabLayer { depth_frac: 0.4,  color: [0.72, 0.58, 0.40] },
        SlabLayer { depth_frac: 0.7,  color: [0.78, 0.65, 0.45] },
        SlabLayer { depth_frac: 1.0,  color: [0.62, 0.58, 0.52] },
    ]
}

impl Default for SlabDef {
    fn default() -> Self {
        Self {
            depth: default_slab_depth(),
            layers: default_slab_layers(),
        }
    }
}

/// A single depth-color layer in the slab cross-section.
#[derive(Debug, Deserialize)]
pub struct SlabLayer {
    /// Fraction of total slab depth (0.0 = top surface, 1.0 = bottom).
    pub depth_frac: f32,
    pub color: [f32; 3],
}

/// Elliptical water body.
#[derive(Debug, Deserialize)]
pub struct WaterDef {
    pub cx: f32,
    pub cz: f32,
    pub radius_x: f32,
    pub radius_z: f32,
}

/// Road defined as a polyline of (x, z) waypoints.
#[derive(Debug, Deserialize)]
pub struct RoadDef {
    pub points: Vec<[f32; 2]>,
    #[serde(default = "default_road_width")]
    pub width: f32,
}

fn default_road_width() -> f32 { 1.6 }

/// Building definition.
#[derive(Debug, Deserialize)]
pub struct BuildingDef {
    pub cx: f32,
    pub cz: f32,
    pub width: f32,
    pub depth: f32,
    pub height: f32,
    pub color: [f32; 3],
    #[serde(default)]
    pub shape: BuildingShape,
}

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingShape {
    #[default]
    Rect,
    LShape,
    TShape,
}

/// Tree definition with optional overrides.
#[derive(Debug, Deserialize)]
pub struct TreeDef {
    pub x: f32,
    pub z: f32,
    #[serde(default = "default_trunk_radius")]
    pub trunk_radius: f32,
    #[serde(default = "default_trunk_height")]
    pub trunk_height: f32,
    #[serde(default = "default_canopy_radius")]
    pub canopy_radius: f32,
    #[serde(default = "default_canopy_height")]
    pub canopy_height: f32,
    #[serde(default = "default_trunk_color")]
    pub trunk_color: [f32; 3],
    #[serde(default = "default_leaf_color")]
    pub leaf_color: [f32; 3],
}

fn default_trunk_radius() -> f32 { 0.12 }
fn default_trunk_height() -> f32 { 0.8 }
fn default_canopy_radius() -> f32 { 0.8 }
fn default_canopy_height() -> f32 { 1.8 }
fn default_trunk_color() -> [f32; 3] { [0.45, 0.30, 0.18] }
fn default_leaf_color() -> [f32; 3] { [0.30, 0.50, 0.22] }

// ---------------------------------------------------------------------------
// Terrain height and color functions
// ---------------------------------------------------------------------------

/// Compute terrain height at (x, z) using Gaussian RBF sum over control points.
///
/// height = sum_i( amplitude_i * exp( -dist^2 / (2 * sigma_i^2) ) )
pub fn terrain_height_rbf(x: f32, z: f32, control_points: &[TerrainControlPoint]) -> f32 {
    let mut h = 0.0_f32;
    for cp in control_points {
        let dx = x - cp.x;
        let dz = z - cp.z;
        let dist_sq = dx * dx + dz * dz;
        h += cp.amplitude * (-dist_sq / (2.0 * cp.sigma * cp.sigma)).exp();
    }
    h
}

/// Determine terrain color from height using sorted color zones.
/// Zones are evaluated in order; the first zone whose max_height >= y is picked.
/// Between adjacent zones, colors are linearly interpolated.
/// If no zone matches, `base_color` is returned.
pub fn terrain_color_from_zones(
    y: f32,
    zones: &[ColorZone],
    base_color: [f32; 3],
) -> [f32; 3] {
    if zones.is_empty() {
        return base_color;
    }

    // Find first zone where max_height >= y
    for (i, zone) in zones.iter().enumerate() {
        if y <= zone.max_height {
            if i == 0 {
                return zone.color;
            }
            // Interpolate between previous zone and this one
            let prev = &zones[i - 1];
            let range = zone.max_height - prev.max_height;
            if range.abs() < 1e-6 {
                return zone.color;
            }
            let t = (y - prev.max_height) / range;
            return lerp_color(prev.color, zone.color, t);
        }
    }

    // y is above all zones — use base_color
    base_color
}

/// Interpolate slab layer color from depth fraction.
pub fn slab_layer_color(depth_frac: f32, layers: &[SlabLayer]) -> [f32; 3] {
    if layers.is_empty() {
        return [0.5, 0.5, 0.5];
    }
    if layers.len() == 1 {
        return layers[0].color;
    }

    let f = depth_frac.clamp(0.0, 1.0);

    for i in 1..layers.len() {
        if f <= layers[i].depth_frac {
            let prev = &layers[i - 1];
            let range = layers[i].depth_frac - prev.depth_frac;
            if range.abs() < 1e-6 {
                return layers[i].color;
            }
            let t = (f - prev.depth_frac) / range;
            return lerp_color(prev.color, layers[i].color, t);
        }
    }

    layers.last().unwrap().color
}

fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + t * (b[0] - a[0]),
        a[1] + t * (b[1] - a[1]),
        a[2] + t * (b[2] - a[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rbf_zero_at_no_control_points() {
        assert_eq!(terrain_height_rbf(5.0, 3.0, &[]), 0.0);
    }

    #[test]
    fn rbf_peak_at_control_point() {
        let cps = vec![TerrainControlPoint { x: 0.0, z: 0.0, amplitude: 2.0, sigma: 5.0 }];
        let h = terrain_height_rbf(0.0, 0.0, &cps);
        assert!((h - 2.0).abs() < 1e-6);
    }

    #[test]
    fn rbf_decays_with_distance() {
        let cps = vec![TerrainControlPoint { x: 0.0, z: 0.0, amplitude: 1.0, sigma: 1.0 }];
        let h_center = terrain_height_rbf(0.0, 0.0, &cps);
        let h_far = terrain_height_rbf(10.0, 10.0, &cps);
        assert!(h_center > h_far);
        assert!(h_far < 0.01);
    }

    #[test]
    fn rbf_negative_amplitude() {
        let cps = vec![TerrainControlPoint { x: 0.0, z: 0.0, amplitude: -1.0, sigma: 3.0 }];
        let h = terrain_height_rbf(0.0, 0.0, &cps);
        assert!((h - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn color_zones_empty_returns_base() {
        let c = terrain_color_from_zones(0.5, &[], [1.0, 0.0, 0.0]);
        assert_eq!(c, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn color_zones_first_zone() {
        let zones = vec![
            ColorZone { max_height: 0.0, color: [0.0, 0.0, 1.0] },
            ColorZone { max_height: 1.0, color: [0.0, 1.0, 0.0] },
        ];
        let c = terrain_color_from_zones(-0.5, &zones, [1.0, 1.0, 1.0]);
        assert_eq!(c, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn color_zones_interpolation() {
        let zones = vec![
            ColorZone { max_height: 0.0, color: [0.0, 0.0, 0.0] },
            ColorZone { max_height: 1.0, color: [1.0, 1.0, 1.0] },
        ];
        let c = terrain_color_from_zones(0.5, &zones, [0.0, 0.0, 0.0]);
        assert!((c[0] - 0.5).abs() < 1e-6);
        assert!((c[1] - 0.5).abs() < 1e-6);
        assert!((c[2] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn slab_layer_single() {
        let layers = vec![SlabLayer { depth_frac: 0.0, color: [1.0, 0.0, 0.0] }];
        assert_eq!(slab_layer_color(0.5, &layers), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn slab_layer_interpolation() {
        let layers = vec![
            SlabLayer { depth_frac: 0.0, color: [0.0, 0.0, 0.0] },
            SlabLayer { depth_frac: 1.0, color: [1.0, 1.0, 1.0] },
        ];
        let c = slab_layer_color(0.5, &layers);
        assert!((c[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn deserialize_minimal_scene() {
        let json = r#"{
            "terrain": {},
            "buildings": [
                {"cx": 0, "cz": 0, "width": 2, "depth": 2, "height": 5, "color": [1,0,0]}
            ]
        }"#;
        let scene: DioramaScene = serde_json::from_str(json).unwrap();
        assert_eq!(scene.terrain.half_size, 18.0);
        assert_eq!(scene.buildings.len(), 1);
        assert_eq!(scene.buildings[0].shape, BuildingShape::Rect);
    }
}
