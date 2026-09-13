/// Bound render cost independently of display density and browser zoom.
pub fn render_size(width: f64, height: f64, pixel_ratio: f64) -> (u32, u32) {
    let width = width.max(1.0);
    let height = height.max(1.0);
    let ratio = pixel_ratio.clamp(1.0, 2.0)
        .min((3_000_000.0 / (width * height)).sqrt())
        .min(4096.0 / width.max(height));
    ((width * ratio).floor().max(1.0) as u32,
     (height * ratio).floor().max(1.0) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn density_is_sharp_but_pixel_cost_is_bounded() {
        assert_eq!(render_size(390.0, 600.0, 3.0), (780, 1200));
        for (w,h,dpr) in [(3840.0,2160.0,2.0),(8000.0,300.0,3.0),(0.0,0.0,1.0)] {
            let (x,y)=render_size(w,h,dpr);
            assert!(x>0 && y>0 && x<=4096 && y<=4096);
            assert!(x as u64*y as u64<=3_000_000);
        }
    }
}
