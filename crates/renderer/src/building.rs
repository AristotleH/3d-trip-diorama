use diorama_core::math::Vec2;
#[cfg(test)]
use diorama_core::triangulate::triangulate;

use crate::vertex::{Vertex, MATERIAL_BUILDING_ROOF, MATERIAL_BUILDING_WALL};

/// Extrude a CCW footprint into a building with material-space façade coordinates
/// and a shallow roof cap. The shader uses these coordinates to generate windows,
/// floor bands, and roof texture consistently for arbitrary footprints.
pub fn extrude_building(
    footprint: &[Vec2],
    height: f32,
    base_y: f32,
    color: [f32; 3],
) -> (Vec<Vertex>, Vec<u32>) {
    extrude_building_with_holes(footprint, &[], height, base_y, color)
}

pub fn extrude_building_with_holes(
    footprint: &[Vec2], holes: &[Vec<Vec2>], height: f32, base_y: f32, color: [f32; 3],
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let n = footprint.len();
    if n < 3 { return (vertices, indices); }

    let top_y = base_y + height;
    let cap_height = (height * 0.025).clamp(0.08, 0.18);
    let roof_y = top_y + cap_height;
    let roof_color = [
        (color[0] * 0.88 + 0.10).min(1.0),
        (color[1] * 0.88 + 0.10).min(1.0),
        (color[2] * 0.88 + 0.10).min(1.0),
    ];
    let top_tris = diorama_core::triangulate::triangulate_with_holes(footprint,holes);
    let all_points: Vec<_> = footprint.iter().chain(holes.iter().flatten()).copied().collect();

    // Roof deck sits above a shallow cap, giving buildings a readable silhouette.
    let roof_start = vertices.len() as u32;
    for p in &all_points {
        vertices.push(Vertex::new(
            [p.x, roof_y, p.y], [0.0, 1.0, 0.0], roof_color,
            [p.x, p.y], MATERIAL_BUILDING_ROOF,
        ));
    }
    for idx in &top_tris { indices.push(roof_start + idx); }

    // Bottom face.
    let bottom_start = vertices.len() as u32;
    for p in &all_points {
        vertices.push(Vertex::new(
            [p.x, base_y, p.y], [0.0, -1.0, 0.0], color,
            [p.x, p.y], MATERIAL_BUILDING_ROOF,
        ));
    }
    for tri in top_tris.chunks(3) {
        indices.extend_from_slice(&[bottom_start + tri[0], bottom_start + tri[2], bottom_start + tri[1]]);
    }

    for ring in std::iter::once(footprint).chain(holes.iter().map(|h|h.as_slice())) {
    let n=ring.len();
    let mut perimeter_u = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let a = ring[i];
        let b = ring[j];
        let dx = b.x - a.x;
        let dz = b.y - a.y;
        let len = (dx * dx + dz * dz).sqrt();
        let normal = if len > 1e-6 { [dz / len, 0.0, -dx / len] } else { [0.0, 0.0, 1.0] };

        // Main façade. U runs continuously around the footprint and V is height
        // above ground, so procedural bays align without per-building textures.
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&[
            Vertex::new([a.x, base_y, a.y], normal, color, [perimeter_u, 0.0], MATERIAL_BUILDING_WALL),
            Vertex::new([b.x, base_y, b.y], normal, color, [perimeter_u + len, 0.0], MATERIAL_BUILDING_WALL),
            Vertex::new([b.x, top_y, b.y], normal, color, [perimeter_u + len, height], MATERIAL_BUILDING_WALL),
            Vertex::new([a.x, top_y, a.y], normal, color, [perimeter_u, height], MATERIAL_BUILDING_WALL),
        ]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

        // Roof cap side doubles as a cornice/parapet band.
        let cap = vertices.len() as u32;
        vertices.extend_from_slice(&[
            Vertex::new([a.x, top_y, a.y], normal, roof_color, [perimeter_u, 0.0], MATERIAL_BUILDING_ROOF),
            Vertex::new([b.x, top_y, b.y], normal, roof_color, [perimeter_u + len, 0.0], MATERIAL_BUILDING_ROOF),
            Vertex::new([b.x, roof_y, b.y], normal, roof_color, [perimeter_u + len, cap_height], MATERIAL_BUILDING_ROOF),
            Vertex::new([a.x, roof_y, a.y], normal, roof_color, [perimeter_u, cap_height], MATERIAL_BUILDING_ROOF),
        ]);
        indices.extend_from_slice(&[cap, cap + 1, cap + 2, cap, cap + 2, cap + 3]);
        perimeter_u += len;
    }
    }
    (vertices, indices)
}

pub fn l_shape_footprint(cx: f32, cz: f32, w: f32, d: f32, notch_frac: f32) -> Vec<Vec2> {
    let hw = w / 2.0; let hd = d / 2.0; let nw = hw * notch_frac; let nd = hd * notch_frac;
    vec![Vec2::new(cx-hw,cz-hd),Vec2::new(cx+hw,cz-hd),Vec2::new(cx+hw,cz+hd-nd),Vec2::new(cx+hw-nw,cz+hd-nd),Vec2::new(cx+hw-nw,cz+hd),Vec2::new(cx-hw,cz+hd)]
}

pub fn rect_footprint(cx: f32, cz: f32, w: f32, d: f32) -> Vec<Vec2> {
    let hw=w/2.0; let hd=d/2.0;
    vec![Vec2::new(cx-hw,cz-hd),Vec2::new(cx+hw,cz-hd),Vec2::new(cx+hw,cz+hd),Vec2::new(cx-hw,cz+hd)]
}

pub fn t_shape_footprint(cx: f32, cz: f32, w: f32, d: f32) -> Vec<Vec2> {
    let hw=w/2.0; let hd=d/2.0; let hsw=w*0.35/2.0; let top_d=d*0.35;
    vec![Vec2::new(cx-hsw,cz-hd),Vec2::new(cx+hsw,cz-hd),Vec2::new(cx+hsw,cz+hd-top_d),Vec2::new(cx+hw,cz+hd-top_d),Vec2::new(cx+hw,cz+hd),Vec2::new(cx-hw,cz+hd),Vec2::new(cx-hw,cz+hd-top_d),Vec2::new(cx-hsw,cz+hd-top_d)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courtyard_roof_leaves_the_hole_open_and_has_inward_facing_walls() {
        let outer=rect_footprint(0.0,0.0,10.0,10.0);
        let mut hole=rect_footprint(0.0,0.0,4.0,4.0);hole.reverse();
        let (v,indices)=extrude_building_with_holes(&outer,&[hole],8.0,0.0,[0.8,0.7,0.6]);
        let top=v.iter().map(|v|v.position[1]).fold(0.0_f32,f32::max);
        let mut roof_area=0.0;
        for tri in indices.chunks_exact(3) {
            let a=v[tri[0] as usize].position;let b=v[tri[1] as usize].position;let c=v[tri[2] as usize].position;
            if a[1]==top&&b[1]==top&&c[1]==top {
                roof_area+=((b[0]-a[0])*(c[2]-a[2])-(b[2]-a[2])*(c[0]-a[0]))*0.5;
                let x=(a[0]+b[0]+c[0])/3.0;let z=(a[2]+b[2]+c[2])/3.0;
                assert!(x.abs()>=2.0 || z.abs()>=2.0);
            }
        }
        assert!((roof_area-84.0).abs()<1e-5);
        assert!(v.iter().any(|v|v.material==MATERIAL_BUILDING_WALL&&v.position[0]==2.0&&v.normal[0]==-1.0));
    }

    #[test]
    fn building_generates_facade_coordinates_and_roof_cap() {
        let footprint = rect_footprint(0.0, 0.0, 4.0, 2.0);
        let (vertices, indices) = extrude_building(&footprint, 8.0, 1.0, [0.8, 0.7, 0.6]);
        assert!(!indices.is_empty());
        assert!(vertices.iter().any(|v| v.material == MATERIAL_BUILDING_WALL && v.uv[1] == 8.0));
        assert!(vertices.iter().any(|v| v.material == MATERIAL_BUILDING_ROOF && v.position[1] > 9.0));
    }
}

#[cfg(test)]
mod osm_tests {
    use super::*;
    #[test]
    fn real_mission_bay_courtyards_have_complete_roofs_with_correct_area() {
        let scene: diorama_core::schema::DioramaScene=serde_json::from_str(
            include_str!("../../../tests/fixtures/mission-bay-courtyard-scene.json")
        ).unwrap();
        let area=|ring: &[Vec2]| -> f32 {
            ring.iter().enumerate().map(|(i,a)|{let b=ring[(i+1)%ring.len()];a.x*b.y-a.y*b.x}).sum::<f32>().abs()*0.5
        };
        assert!(scene.buildings.iter().any(|b|!b.holes.is_empty()));
        for b in &scene.buildings {
            let outer: Vec<_>=b.footprint.iter().map(|p|Vec2::new(p[0],p[1])).collect();
            let holes: Vec<Vec<_>>=b.holes.iter().map(|h|h.iter().map(|p|Vec2::new(p[0],p[1])).collect()).collect();
            let expected=area(&outer)-holes.iter().map(|h|area(h)).sum::<f32>();
            let all: Vec<_>=outer.iter().chain(holes.iter().flatten()).copied().collect();
            let indices=diorama_core::triangulate::triangulate_with_holes(&outer,&holes);
            let actual=indices.chunks_exact(3).map(|t|area(&[all[t[0] as usize],all[t[1] as usize],all[t[2] as usize]])).sum::<f32>();
            assert!((actual-expected).abs()<0.001,"roof area mismatch at {}, {}",b.cx,b.cz);
        }
    }
    #[test]
    fn clipped_buildings_have_complete_roofs_and_walls_at_the_slab_edge() {
        let scene: diorama_core::schema::DioramaScene = serde_json::from_str(
            include_str!("../../../tests/fixtures/edge-buildings-scene.json")
        ).unwrap();
        assert_eq!(scene.buildings.len(),3);
        for b in &scene.buildings {
            let footprint: Vec<_> = b.footprint.iter().map(|p| Vec2::new(p[0],p[1])).collect();
            assert_eq!(triangulate(&footprint).len(),(footprint.len()-2)*3);
            let (vertices,indices)=extrude_building(&footprint,b.height,0.0,b.color);
            assert!(vertices.iter().all(|v|v.position[0].abs()<=36.0 && v.position[2].abs()<=36.0));
            assert!(indices.chunks_exact(3).any(|triangle| {
                let points: Vec<_> = triangle.iter().map(|&i|&vertices[i as usize]).collect();
                points.iter().all(|v|v.material==MATERIAL_BUILDING_WALL) &&
                    (points.iter().all(|v|v.position[0]==36.0) ||
                     points.iter().all(|v|v.position[2].abs()==36.0))
            }), "the cut edge needs a closed vertical wall");
        }
    }
    #[test]
    fn mission_bay_water_triangulates_without_missing_or_reversed_faces() {
        let scene: diorama_core::schema::DioramaScene = serde_json::from_str(
            include_str!("../../../tests/fixtures/mission-bay-water-scene.json")
        ).unwrap();
        let mut total_area = 0.0_f32;
        assert!(!scene.surfaces.is_empty());
        for surface in &scene.surfaces {
            let points: Vec<_> = surface.points.iter().map(|p| Vec2::new(p[0],p[1])).collect();
            let indices = triangulate(&points);
            assert_eq!(indices.len(), (points.len()-2)*3);
            for triangle in indices.chunks_exact(3) {
                let a=points[triangle[0] as usize];
                let b=points[triangle[1] as usize];
                let c=points[triangle[2] as usize];
                let area=((b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x))*0.5;
                assert!(area>0.0, "water must face the camera like terrain");
                total_area+=area;
            }
        }
        assert!((total_area-1852.76).abs()<0.1);
    }
    #[test]
    fn live_osm_export_deserializes_and_triangulates_every_footprint() {
        let scene: diorama_core::schema::DioramaScene = serde_json::from_str(
            include_str!("../../../tests/fixtures/tokyo-osm-scene.json")
        ).unwrap();
        assert!(!scene.buildings.is_empty());
        for b in &scene.buildings {
            let p: Vec<_> = b.footprint.iter().map(|v| Vec2::new(v[0], v[1])).collect();
            assert_eq!(triangulate(&p).len(), (p.len()-2)*3, "incomplete building roof at {}, {}", b.cx, b.cz);
            let (v,i)=extrude_building(&p,b.height,0.0,b.color);
            assert!(i.iter().all(|&idx| (idx as usize)<v.len()));
            assert!(v.iter().all(|v| v.position.iter().all(|x| x.is_finite())));
        }
        for surface in &scene.surfaces {
            let p: Vec<_> = surface.points.iter().map(|v| Vec2::new(v[0], v[1])).collect();
            assert_eq!(triangulate(&p).len(),(p.len()-2)*3);
        }
    }
}
