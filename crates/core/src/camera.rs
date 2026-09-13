use crate::math::{Mat4, Vec3};

pub struct OrbitCamera {
    pub target: Vec3,
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
    pub max_distance: f32,
}

impl OrbitCamera {
    pub fn new() -> Self {
        Self {
            target: Vec3::new(0.0, 0.0, 0.0),
            azimuth: std::f32::consts::FRAC_PI_4,
            elevation: 55.0_f32.to_radians(),
            distance: 40.0,
            max_distance: 100.0,
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

    pub fn fit_bounds(&mut self, half_size: f32, bottom: f32, top: f32, aspect: f32) {
        self.target = Vec3::new(0.0, (bottom + top) * 0.5, 0.0);
        let direction = (self.eye() - self.target).normalize();
        let right = Vec3::new(0.0, 1.0, 0.0).cross(direction).normalize();
        let up = direction.cross(right);
        let tan_y = 22.5_f32.to_radians().tan();
        let tan_x = tan_y * aspect;
        let mut distance = 5.0_f32;
        for x in [-half_size, half_size] {
            for y in [bottom, top] {
                for z in [-half_size, half_size] {
                    let p = Vec3::new(x, y, z) - self.target;
                    distance = distance.max(p.dot(direction) + 1.12 *
                        (p.dot(right).abs()/tan_x).max(p.dot(up).abs()/tan_y));
                }
            }
        }
        self.distance = distance;
        self.max_distance = distance * 3.0;
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        // A fixed 0.1 near plane wastes depth precision when viewing a whole
        // neighborhood, causing thin ground overlays to fight with terrain.
        let near = (self.distance * 0.01).max(0.1);
        Mat4::perspective(45.0_f32.to_radians(), aspect, near, (self.max_distance * 4.0).max(200.0))
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
        self.distance = self.distance.clamp(5.0, self.max_distance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitted_scene_stays_inside_portrait_and_landscape_viewports() {
        for aspect in [0.5, 1.0, 1.65, 2.4] {
            let mut camera = OrbitCamera::new();
            camera.fit_bounds(36.0, -1.8, 8.0, aspect);
            let m = (camera.projection_matrix(aspect) * camera.view_matrix()).cols;
            let mut extent=0.0_f32;
            for x in [-36.0,36.0] { for y in [-1.8,8.0] { for z in [-36.0,36.0] {
                let p=[x,y,z,1.0];
                let c: Vec<f32>=(0..4).map(|r|(0..4).map(|i|m[i][r]*p[i]).sum()).collect();
                assert!(c[3]>0.0);
                extent=extent.max((c[0]/c[3]).abs()).max((c[1]/c[3]).abs());
            }}}
            assert!(extent<1.0 && extent>0.8, "scene should fill viewport without cropping");
        }
    }

    #[test]
    fn ground_overlays_retain_depth_separation_at_neighborhood_zoom() {
        let mut camera = OrbitCamera::new();
        camera.max_distance = 500.0;
        for distance in [40.0, 150.0, 500.0] {
            camera.distance = distance;
            let projection = camera.projection_matrix(1.5);
            let depth = |d: f32| -projection.cols[2][2] + projection.cols[3][2] / d;
            // Worst allowed viewing angle, including the far edge of the slab.
            let far_edge = distance + 51.0;
            let ground = depth(far_edge);
            let overlay = depth(far_edge - 0.04 * 10.0_f32.to_radians().sin());
            assert!(ground.to_bits() >= overlay.to_bits() + 2,
                "overlays need distinct Depth32Float values at distance {distance}");
        }
    }
}
