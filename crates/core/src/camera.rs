use crate::math::{Mat4, Vec3};

pub struct OrbitCamera {
    pub target: Vec3,
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
}

impl OrbitCamera {
    pub fn new() -> Self {
        Self {
            target: Vec3::new(0.0, 0.0, 0.0),
            azimuth: std::f32::consts::FRAC_PI_4,
            elevation: 55.0_f32.to_radians(),
            distance: 40.0,
        }
    }

    pub fn eye(&self) -> Vec3 {
        let cos_el = self.elevation.cos();
        Vec3::new(
            self.target.x + self.distance * cos_el * self.azimuth.cos(),
            self.target.y + self.distance * self.elevation.sin(),
            self.target.z + self.distance * cos_el * self.azimuth.sin(),
        )
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.eye(), self.target, Vec3::new(0.0, 1.0, 0.0))
    }

    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        self.azimuth += delta_x * 0.01;
        self.elevation += delta_y * 0.01;
        let min_el = 10.0_f32.to_radians();
        let max_el = 80.0_f32.to_radians();
        self.elevation = self.elevation.clamp(min_el, max_el);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance *= 1.0 + delta * 0.001;
        self.distance = self.distance.clamp(5.0, 100.0);
    }
}
