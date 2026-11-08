//! Reine Szenen-Logik des Viewers: Achsen-Umrechnung, Bounding-Box/Kamera-
//! Einpassung, Gitter (wie `THREE.GridHelper`) und Farbraum-Umrechnung.

use super::orbit::Orbit;
use super::types::Gr2Mesh;
use glam::{Mat4, Vec3};

/// Granny/Metin2-Modelle sind Z-oben, die Szene ist Y-oben - ohne diese
/// Drehung (−90° um X) läge jedes Modell auf dem Rücken.
pub fn z_up_to_y_up() -> Mat4 {
    Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
}

/// Achsenparallele Box der (bereits gedrehten) Modellpunkte; `None` wenn leer.
pub fn bounds(meshes: &[Gr2Mesh], rot: Mat4) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut any = false;
    for m in meshes {
        for p in m.vertices.chunks_exact(3) {
            let v = rot.transform_point3(Vec3::new(p[0], p[1], p[2]));
            min = min.min(v);
            max = max.max(v);
            any = true;
        }
    }
    any.then_some((min, max))
}

/// Ergebnis der Kamera-Einpassung (wie im TS: Modell in den Ursprung
/// verschieben, Kamera bei (0.5, 0.5, 0.8)·Diagonale, near/far relativ).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub model: Mat4,
    pub camera_position: Vec3,
    pub near: f32,
    pub far: f32,
}

pub fn fit(meshes: &[Gr2Mesh]) -> Fit {
    let rot = z_up_to_y_up();
    let (center, size) = match bounds(meshes, rot) {
        Some((min, max)) => ((min + max) * 0.5, (max - min).length().max(1.0)),
        None => (Vec3::ZERO, 1.0),
    };
    Fit {
        model: Mat4::from_translation(-center) * rot,
        camera_position: Vec3::new(size * 0.5, size * 0.5, size * 0.8),
        near: size / 100.0,
        far: size * 100.0,
    }
}

impl Fit {
    pub fn apply(&self, orbit: &mut Orbit) {
        orbit.position = self.camera_position;
        orbit.near = self.near;
        orbit.far = self.far;
        orbit.target = Vec3::ZERO;
        orbit.update();
    }
}

/// `new THREE.GridHelper(200, 20, 0x444444, 0x2a2a2a)`: Linien in der
/// XZ-Ebene als (x, y, z, r, g, b)-Paare; Farben bereits im sRGB-Ausgaberaum.
pub fn grid_lines(size: f32, divisions: u32, center_color: u32, grid_color: u32) -> Vec<f32> {
    let step = size / divisions as f32;
    let half = size / 2.0;
    let c1 = hex_rgb(center_color);
    let c2 = hex_rgb(grid_color);
    let mut out = Vec::with_capacity((divisions as usize + 1) * 4 * 6);
    for i in 0..=divisions {
        let k = -half + i as f32 * step;
        let c = if i == divisions / 2 { c1 } else { c2 };
        for p in [[-half, 0.0, k], [half, 0.0, k], [k, 0.0, -half], [k, 0.0, half]] {
            out.extend_from_slice(&p);
            out.extend_from_slice(&c);
        }
    }
    out
}

pub fn hex_rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}

/// sRGB → linear (three.js wandelt Hex-Materialfarben genauso um).
pub fn srgb_to_linear(c: f32) -> f32 {
    if c < 0.04045 {
        c * 0.077_399_38
    } else {
        ((c * 0.947_867_3) + 0.052_132_7).powf(2.4)
    }
}

pub fn hex_linear(hex: u32) -> [f32; 3] {
    hex_rgb(hex).map(srgb_to_linear)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(vertices: Vec<f32>) -> Gr2Mesh {
        Gr2Mesh { vertices, ..Default::default() }
    }

    #[test]
    fn z_up_becomes_y_up() {
        let v = z_up_to_y_up().transform_point3(Vec3::new(1.0, 2.0, 3.0));
        assert!((v - Vec3::new(1.0, 3.0, -2.0)).length() < 1e-5);
    }

    #[test]
    fn fit_centers_model_and_places_camera() {
        // Z-oben: Box x 0..2, y 0..4, z 0..10
        let f = fit(&[mesh(vec![0.0, 0.0, 0.0, 2.0, 4.0, 10.0])]);
        let a = f.model.transform_point3(Vec3::new(0.0, 0.0, 0.0));
        let b = f.model.transform_point3(Vec3::new(2.0, 4.0, 10.0));
        assert!(((a + b) * 0.5).length() < 1e-4, "Mitte im Ursprung");
        // Höhe (z) zeigt nach +Y
        assert!((b.y - a.y - 10.0).abs() < 1e-4);
        let size = (4.0f32 + 16.0 + 100.0).sqrt();
        assert!((f.camera_position - Vec3::new(size * 0.5, size * 0.5, size * 0.8)).length() < 1e-4);
        assert!((f.near - size / 100.0).abs() < 1e-5);
        assert!((f.far - size * 100.0).abs() < 1e-2);
    }

    #[test]
    fn fit_of_tiny_or_empty_model_uses_min_size_one() {
        let f = fit(&[mesh(vec![])]);
        assert_eq!(f.camera_position, Vec3::new(0.5, 0.5, 0.8));
        let f = fit(&[mesh(vec![1.0, 1.0, 1.0])]);
        assert_eq!(f.camera_position, Vec3::new(0.5, 0.5, 0.8));
        assert!(f.model.transform_point3(Vec3::ONE).length() < 1e-5);
    }

    #[test]
    fn grid_has_center_lines_in_center_color() {
        let g = grid_lines(200.0, 20, 0x444444, 0x2a2a2a);
        assert_eq!(g.len(), 21 * 4 * 6);
        // Linie i = 10 liegt bei 0 und hat die Mittelfarbe.
        let first_center = 10 * 4 * 6;
        assert_eq!(&g[first_center..first_center + 3], &[-100.0, 0.0, 0.0]);
        assert!((g[first_center + 3] - 0x44 as f32 / 255.0).abs() < 1e-6);
        assert!((g[3] - 0x2a as f32 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn srgb_conversion() {
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-5);
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert!((hex_linear(0xcfcfcf)[0] - 0.624).abs() < 0.002);
    }
}
