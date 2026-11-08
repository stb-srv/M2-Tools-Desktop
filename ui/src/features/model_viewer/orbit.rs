//! Orbit-Steuerung nach dem Vorbild von three.js `OrbitControls`
//! (Linksziehen = drehen, Mausrad/Mittelziehen = zoomen, Rechtsziehen bzw.
//! Strg/Shift/Cmd + Linksziehen = verschieben), mit `enableDamping` und dem
//! Standard-Dämpfungsfaktor 0.05. Reine Mathematik, damit sie nativ testbar ist.

use glam::{Mat4, Vec3};
use std::f32::consts::PI;

const EPS: f32 = 0.000_001;
const DAMPING: f32 = 0.05;

/// Perspektivkamera + Orbit-Zustand (entspricht `PerspectiveCamera` +
/// `OrbitControls` aus dem TS-Code).
#[derive(Clone, Debug)]
pub struct Orbit {
    pub position: Vec3,
    pub target: Vec3,
    /// Vertikales Sichtfeld in Grad.
    pub fov_deg: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    delta_theta: f32,
    delta_phi: f32,
    scale: f32,
    pan_offset: Vec3,
}

impl Orbit {
    /// `new PerspectiveCamera(50, aspect, 0.1, 10000)` an (100, 100, 200).
    pub fn new(aspect: f32) -> Self {
        Self {
            position: Vec3::new(100.0, 100.0, 200.0),
            target: Vec3::ZERO,
            fov_deg: 50.0,
            aspect,
            near: 0.1,
            far: 10000.0,
            delta_theta: 0.0,
            delta_phi: 0.0,
            scale: 1.0,
            pan_offset: Vec3::ZERO,
        }
    }

    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, Vec3::Y)
    }

    pub fn projection(&self) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov_deg.to_radians(), self.aspect, self.near, self.far)
    }

    /// Weltmatrix der Kamera (Spalte 0 = rechts, Spalte 1 = oben).
    fn camera_matrix(&self) -> Mat4 {
        self.view().inverse()
    }

    /// Linksziehen um (dx, dy) Pixel bei gegebener Elementhöhe.
    pub fn rotate(&mut self, dx: f32, dy: f32, client_height: f32) {
        let h = client_height.max(1.0);
        self.delta_theta -= 2.0 * PI * dx / h;
        self.delta_phi -= 2.0 * PI * dy / h;
    }

    /// Verschieben um (dx, dy) Pixel (screenSpacePanning wie three.js-Standard).
    pub fn pan(&mut self, dx: f32, dy: f32, client_height: f32) {
        let h = client_height.max(1.0);
        let offset = self.position - self.target;
        let target_distance = offset.length() * (self.fov_deg.to_radians() / 2.0).tan();
        let m = self.camera_matrix();
        let right = m.x_axis.truncate();
        let up = m.y_axis.truncate();
        self.pan_offset += right * (-(2.0 * dx * target_distance / h));
        self.pan_offset += up * (2.0 * dy * target_distance / h);
    }

    fn zoom_scale(delta: f32) -> f32 {
        let normalized = (delta * 0.01).abs();
        0.95f32.powf(normalized)
    }

    /// Positives `delta` = herauszoomen (wie Mausrad nach unten).
    pub fn dolly(&mut self, delta: f32) {
        if delta < 0.0 {
            self.scale *= Self::zoom_scale(delta);
        } else if delta > 0.0 {
            self.scale /= Self::zoom_scale(delta);
        }
    }

    /// Mausrad: `deltaY` wie im WheelEvent, `delta_mode` 0 = Pixel,
    /// 1 = Zeilen, 2 = Seiten; `ctrl` = Pinch-Geste.
    pub fn wheel(&mut self, delta_y: f32, delta_mode: u32, ctrl: bool) {
        let mut d = delta_y;
        match delta_mode {
            1 => d *= 16.0,
            2 => d *= 100.0,
            _ => {}
        }
        if ctrl {
            d *= 10.0;
        }
        self.dolly(d);
    }

    /// Ein Schritt von `controls.update()`; gibt `true` zurück, solange die
    /// Dämpfung noch Bewegung erzeugt.
    pub fn update(&mut self) -> bool {
        let before = self.position;
        let mut offset = self.position - self.target;
        let mut radius = offset.length();
        let (mut theta, mut phi) = if radius == 0.0 {
            (0.0, 0.0)
        } else {
            (offset.x.atan2(offset.z), (offset.y / radius).clamp(-1.0, 1.0).acos())
        };

        theta += self.delta_theta * DAMPING;
        phi += self.delta_phi * DAMPING;
        phi = phi.clamp(EPS, PI - EPS);

        self.target += self.pan_offset * DAMPING;

        radius = (radius * self.scale).max(0.0);

        let sin_phi_r = phi.sin() * radius;
        offset = Vec3::new(sin_phi_r * theta.sin(), phi.cos() * radius, sin_phi_r * theta.cos());
        self.position = self.target + offset;

        self.delta_theta *= 1.0 - DAMPING;
        self.delta_phi *= 1.0 - DAMPING;
        self.pan_offset *= 1.0 - DAMPING;
        self.scale = 1.0;

        (self.position - before).length_squared() > EPS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_without_input_keeps_camera() {
        let mut o = Orbit::new(1.5);
        let p = o.position;
        o.update();
        assert!((o.position - p).length() < 1e-3);
    }

    #[test]
    fn wheel_down_zooms_out_and_up_zooms_in() {
        let mut o = Orbit::new(1.0);
        let r0 = o.position.length();
        o.wheel(100.0, 0, false);
        o.update();
        let r1 = o.position.length();
        assert!(r1 > r0);
        // 0.95^1 -> Radius / 0.95
        assert!((r1 - r0 / 0.95).abs() < 1e-2);
        o.wheel(-100.0, 0, false);
        o.update();
        assert!((o.position.length() - r0).abs() < 1e-2);
    }

    #[test]
    fn line_mode_wheel_is_scaled() {
        let mut a = Orbit::new(1.0);
        let mut b = Orbit::new(1.0);
        a.wheel(3.0, 1, false);
        b.wheel(48.0, 0, false);
        a.update();
        b.update();
        assert!((a.position - b.position).length() < 1e-3);
    }

    #[test]
    fn rotation_is_damped_and_keeps_distance() {
        let mut o = Orbit::new(1.0);
        let r0 = o.position.length();
        o.rotate(100.0, 0.0, 500.0);
        o.update();
        let after_one = o.position;
        assert!((after_one.length() - r0).abs() < 1e-2);
        for _ in 0..500 {
            o.update();
        }
        // Gesamtdrehung konvergiert gegen 2π·100/500 um die Y-Achse.
        let start = Vec3::new(100.0, 100.0, 200.0);
        let a0 = start.x.atan2(start.z);
        let a1 = o.position.x.atan2(o.position.z);
        let expected = -2.0 * PI * 100.0 / 500.0;
        let mut diff = a1 - a0 - expected;
        while diff > PI {
            diff -= 2.0 * PI;
        }
        while diff < -PI {
            diff += 2.0 * PI;
        }
        assert!(diff.abs() < 1e-2, "diff {diff}");
        assert!((o.position.length() - r0).abs() < 1e-2);
    }

    #[test]
    fn polar_angle_is_clamped() {
        let mut o = Orbit::new(1.0);
        o.rotate(0.0, 100_000.0, 100.0);
        for _ in 0..200 {
            o.update();
        }
        assert!(o.position.y > 0.0);
        assert!(o.position.y.is_finite());
    }

    #[test]
    fn pan_moves_target_and_camera_together() {
        let mut o = Orbit::new(1.0);
        let off0 = o.position - o.target;
        o.pan(50.0, 0.0, 500.0);
        for _ in 0..500 {
            o.update();
        }
        assert!(o.target.length() > 1.0);
        assert!(((o.position - o.target) - off0).length() < 1e-2);
    }
}
