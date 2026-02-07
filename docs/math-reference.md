# Math Reference

Key formulas and implementations for the custom math library.

## Vector Operations

### Vec3

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0, z: 1.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0, z: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0 };
    pub const Z: Self = Self { x: 0.0, y: 0.0, z: 1.0 };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            Self::ZERO
        }
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }
}

// Implement Add, Sub, Mul, Div, Neg
```

## Matrix Operations

### Mat4 (Column-Major)

```rust
#[derive(Clone, Copy, Debug)]
pub struct Mat4 {
    // Column-major: cols[col][row]
    pub cols: [[f32; 4]; 4],
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        cols: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    pub fn translation(v: Vec3) -> Self {
        Self {
            cols: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [v.x, v.y, v.z, 1.0],
            ],
        }
    }

    pub fn scale(v: Vec3) -> Self {
        Self {
            cols: [
                [v.x, 0.0, 0.0, 0.0],
                [0.0, v.y, 0.0, 0.0],
                [0.0, 0.0, v.z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_y(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();
        Self {
            cols: [
                [c, 0.0, -s, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [s, 0.0, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    // Similar for rotation_x, rotation_z

    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y / 2.0).tan();
        let nf = 1.0 / (near - far);

        Self {
            cols: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (far + near) * nf, -1.0],
                [0.0, 0.0, 2.0 * far * near * nf, 0.0],
            ],
        }
    }

    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize();
        let r = f.cross(up).normalize();
        let u = r.cross(f);

        Self {
            cols: [
                [r.x, u.x, -f.x, 0.0],
                [r.y, u.y, -f.y, 0.0],
                [r.z, u.z, -f.z, 0.0],
                [-r.dot(eye), -u.dot(eye), f.dot(eye), 1.0],
            ],
        }
    }

    // Matrix multiplication
    pub fn mul(self, other: Self) -> Self {
        let mut result = [[0.0f32; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += self.cols[k][j] * other.cols[i][k];
                }
            }
        }
        Self { cols: result }
    }

    // Transform point (w=1)
    pub fn transform_point(self, v: Vec3) -> Vec3 {
        let w = self.cols[0][3] * v.x + self.cols[1][3] * v.y + self.cols[2][3] * v.z + self.cols[3][3];
        Vec3 {
            x: (self.cols[0][0] * v.x + self.cols[1][0] * v.y + self.cols[2][0] * v.z + self.cols[3][0]) / w,
            y: (self.cols[0][1] * v.x + self.cols[1][1] * v.y + self.cols[2][1] * v.z + self.cols[3][1]) / w,
            z: (self.cols[0][2] * v.x + self.cols[1][2] * v.y + self.cols[2][2] * v.z + self.cols[3][2]) / w,
        }
    }

    // Transform direction (w=0)
    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        Vec3 {
            x: self.cols[0][0] * v.x + self.cols[1][0] * v.y + self.cols[2][0] * v.z,
            y: self.cols[0][1] * v.x + self.cols[1][1] * v.y + self.cols[2][1] * v.z,
            z: self.cols[0][2] * v.x + self.cols[1][2] * v.y + self.cols[2][2] * v.z,
        }
    }

    // For uploading to GPU (wgpu expects [f32; 16])
    pub fn to_cols_array(self) -> [f32; 16] {
        let mut arr = [0.0; 16];
        for i in 0..4 {
            for j in 0..4 {
                arr[i * 4 + j] = self.cols[i][j];
            }
        }
        arr
    }
}
```

## Camera Math

### Orbit Camera

```rust
pub struct OrbitCamera {
    pub center: Vec3,     // Point we're looking at
    pub distance: f32,    // Distance from center
    pub azimuth: f32,     // Horizontal angle (radians)
    pub elevation: f32,   // Vertical angle (radians), 0 = horizon
}

impl OrbitCamera {
    pub fn view_matrix(&self) -> Mat4 {
        let eye = self.eye_position();
        Mat4::look_at(eye, self.center, Vec3::Y)
    }

    pub fn eye_position(&self) -> Vec3 {
        let y = self.distance * self.elevation.sin();
        let horizontal = self.distance * self.elevation.cos();
        let x = horizontal * self.azimuth.sin();
        let z = horizontal * self.azimuth.cos();
        self.center + Vec3::new(x, y, z)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective(
            45.0_f32.to_radians(),  // fov
            aspect,
            0.1,                     // near
            1000.0,                  // far
        )
    }
}
```

## Triangulation (Ear Clipping)

```rust
pub fn triangulate(polygon: &[Vec2]) -> Vec<[usize; 3]> {
    if polygon.len() < 3 {
        return vec![];
    }

    let mut indices: Vec<usize> = (0..polygon.len()).collect();
    let mut triangles = Vec::new();

    while indices.len() > 3 {
        let n = indices.len();
        let mut ear_found = false;

        for i in 0..n {
            let prev = indices[(i + n - 1) % n];
            let curr = indices[i];
            let next = indices[(i + 1) % n];

            let a = polygon[prev];
            let b = polygon[curr];
            let c = polygon[next];

            // Check if this is a convex vertex (ear candidate)
            if !is_convex(a, b, c) {
                continue;
            }

            // Check no other vertices inside this triangle
            let mut is_ear = true;
            for j in 0..n {
                if j == (i + n - 1) % n || j == i || j == (i + 1) % n {
                    continue;
                }
                if point_in_triangle(polygon[indices[j]], a, b, c) {
                    is_ear = false;
                    break;
                }
            }

            if is_ear {
                triangles.push([prev, curr, next]);
                indices.remove(i);
                ear_found = true;
                break;
            }
        }

        if !ear_found {
            // Degenerate polygon, bail out
            break;
        }
    }

    if indices.len() == 3 {
        triangles.push([indices[0], indices[1], indices[2]]);
    }

    triangles
}

fn is_convex(a: Vec2, b: Vec2, c: Vec2) -> bool {
    // Cross product > 0 for CCW winding
    let ab = Vec2::new(b.x - a.x, b.y - a.y);
    let bc = Vec2::new(c.x - b.x, c.y - b.y);
    ab.x * bc.y - ab.y * bc.x > 0.0
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let sign = |p1: Vec2, p2: Vec2, p3: Vec2| -> f32 {
        (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
    };

    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);

    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;

    !(has_neg && has_pos)
}
```

## Geographic Conversions

```rust
use std::f64::consts::PI;

pub fn lat_lng_to_tile(lat: f64, lng: f64, zoom: u8) -> (u32, u32) {
    let n = 2.0_f64.powi(zoom as i32);
    let x = ((lng + 180.0) / 360.0 * n).floor() as u32;
    let lat_rad = lat.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n).floor() as u32;
    (x, y)
}

pub fn tile_to_lat_lng(x: u32, y: u32, zoom: u8) -> (f64, f64) {
    let n = 2.0_f64.powi(zoom as i32);
    let lng = x as f64 / n * 360.0 - 180.0;
    let lat_rad = (PI * (1.0 - 2.0 * y as f64 / n)).sinh().atan();
    let lat = lat_rad.to_degrees();
    (lat, lng)
}

// Web Mercator projection (EPSG:3857)
pub fn lat_lng_to_meters(lat: f64, lng: f64) -> (f64, f64) {
    const EARTH_RADIUS: f64 = 6378137.0;
    let x = lng.to_radians() * EARTH_RADIUS;
    let y = (lat.to_radians() / 2.0 + PI / 4.0).tan().ln() * EARTH_RADIUS;
    (x, y)
}

pub fn haversine_km(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6371.0; // Earth radius in km
    let dlat = (lat2 - lat1).to_radians();
    let dlng = (lng2 - lng1).to_radians();
    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();

    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    R * c
}
```
