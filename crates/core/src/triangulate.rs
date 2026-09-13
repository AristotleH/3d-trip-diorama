use crate::math::Vec2;

/// Indices address outer vertices followed by each hole's vertices.
pub fn triangulate_with_holes(outer: &[Vec2], holes: &[Vec<Vec2>]) -> Vec<u32> {
    if holes.is_empty() { return triangulate(outer); }
    let mut points=outer.to_vec();
    let mut starts=Vec::new();
    for hole in holes {
        if hole.len()<3 { return vec![]; }
        starts.push(points.len());points.extend_from_slice(hole);
    }
    let coordinates: Vec<f64> = points.iter().flat_map(|p|[p.x as f64,p.y as f64]).collect();
    let Ok(mut indices)=earcutr::earcut(&coordinates,&starts,2) else { return vec![]; };
    for triangle in indices.chunks_exact_mut(3) {
        let a=points[triangle[0]];let b=points[triangle[1]];let c=points[triangle[2]];
        if (b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x)<0.0 { triangle.swap(1,2); }
    }
    indices.into_iter().map(|i|i as u32).collect()
}

/// Ear-clipping triangulation for a simple polygon (CCW winding).
/// Returns indices into the input slice forming triangles.
pub fn triangulate(polygon: &[Vec2]) -> Vec<u32> {
    let n = polygon.len();
    if n < 3 {
        return vec![];
    }

    let mut indices: Vec<u32> = Vec::new();
    let mut remaining: Vec<usize> = (0..n).collect();

    while remaining.len() > 3 {
        let len = remaining.len();
        let mut found_ear = false;

        for i in 0..len {
            let prev = remaining[(i + len - 1) % len];
            let curr = remaining[i];
            let next = remaining[(i + 1) % len];

            let a = polygon[prev];
            let b = polygon[curr];
            let c = polygon[next];

            // Must be convex (positive cross product for CCW winding)
            let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            if cross <= 0.0 {
                continue;
            }

            // No other vertex inside this triangle
            let mut ear = true;
            for &ri in &remaining {
                if ri == prev || ri == curr || ri == next {
                    continue;
                }
                if point_in_triangle(polygon[ri], a, b, c) {
                    ear = false;
                    break;
                }
            }

            if ear {
                indices.push(prev as u32);
                indices.push(curr as u32);
                indices.push(next as u32);
                remaining.remove(i);
                found_ear = true;
                break;
            }
        }

        if !found_ear {
            break;
        }
    }

    if remaining.len() == 3 {
        indices.push(remaining[0] as u32);
        indices.push(remaining[1] as u32);
        indices.push(remaining[2] as u32);
    }

    indices
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

fn sign(p1: Vec2, p2: Vec2, p3: Vec2) -> f32 {
    (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangulate_square() {
        let square = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let tris = triangulate(&square);
        assert_eq!(tris.len(), 6); // 2 triangles × 3 indices
    }

    #[test]
    fn triangulate_triangle() {
        let tri = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.5, 1.0),
        ];
        let tris = triangulate(&tri);
        assert_eq!(tris.len(), 3);
    }
}
