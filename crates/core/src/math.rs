use std::ops::{Add, Sub, Mul, Neg};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len < 1e-10 {
            return self;
        }
        self * (1.0 / len)
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

/// Column-major 4x4 matrix. `cols[i]` is the i-th column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    pub cols: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn identity() -> Self {
        let mut cols = [[0.0f32; 4]; 4];
        cols[0][0] = 1.0;
        cols[1][1] = 1.0;
        cols[2][2] = 1.0;
        cols[3][3] = 1.0;
        Self { cols }
    }

    /// Flatten to a 16-element array in column-major order (what WebGPU expects).
    pub fn to_cols_array(&self) -> [f32; 16] {
        let c = &self.cols;
        [
            c[0][0], c[0][1], c[0][2], c[0][3],
            c[1][0], c[1][1], c[1][2], c[1][3],
            c[2][0], c[2][1], c[2][2], c[2][3],
            c[3][0], c[3][1], c[3][2], c[3][3],
        ]
    }

    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize();
        let r = f.cross(up).normalize();
        let u = r.cross(f);

        let mut m = Self::identity();
        m.cols[0][0] = r.x;
        m.cols[1][0] = r.y;
        m.cols[2][0] = r.z;
        m.cols[3][0] = -r.dot(eye);

        m.cols[0][1] = u.x;
        m.cols[1][1] = u.y;
        m.cols[2][1] = u.z;
        m.cols[3][1] = -u.dot(eye);

        m.cols[0][2] = -f.x;
        m.cols[1][2] = -f.y;
        m.cols[2][2] = -f.z;
        m.cols[3][2] = f.dot(eye);

        m
    }

    /// Perspective projection mapping to wgpu depth range [0, 1].
    pub fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y_rad / 2.0).tan();
        let mut m = Self { cols: [[0.0; 4]; 4] };
        m.cols[0][0] = f / aspect;
        m.cols[1][1] = f;
        m.cols[2][2] = far / (near - far);
        m.cols[2][3] = -1.0;
        m.cols[3][2] = (near * far) / (near - far);
        m
    }
}

impl Mul for Mat4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut out = Self { cols: [[0.0; 4]; 4] };
        for c in 0..4 {
            for r in 0..4 {
                out.cols[c][r] = self.cols[0][r] * rhs.cols[c][0]
                    + self.cols[1][r] * rhs.cols[c][1]
                    + self.cols[2][r] * rhs.cols[c][2]
                    + self.cols[3][r] * rhs.cols[c][3];
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_cross() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = x.cross(y);
        assert!((z.x).abs() < 1e-6);
        assert!((z.y).abs() < 1e-6);
        assert!((z.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mat4_identity_mul() {
        let i = Mat4::identity();
        let p = Mat4::perspective(1.0, 1.0, 0.1, 100.0);
        let result = i * p;
        assert_eq!(result, p);
    }
}
