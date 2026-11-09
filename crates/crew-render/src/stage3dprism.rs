//! `prism`, the scene the prism theme draws behind its panes (`stage3d`):
//! faceted glass prisms and two gems drifting at three depths across the
//! window, tumbling slowly, fading into the page through a fog of the page's
//! own colour, under a key light in the theme's accent and a rim light in its
//! activity colour, with a thin halo in the ring's colours low behind them.
//!
//! Unlike `orbit`, whose one bright knot sits behind one pane, it fills the
//! frame evenly at low contrast, so every pane of a grid has depth behind it.
//! Each prism is placed at a fraction of the frustum's half-width at its
//! depth, not at a world position, so a tall or an ultrawide window keeps
//! every prism in frame.
//!
//! Every motion is a whole number of turns per turn of the clock, so the
//! picture at phase 1 is the picture at phase 0 and the wrap never jumps.
use std::f32::consts::TAU;

use threers::cameras::Camera as _;
use threers::{
    compute_vertex_normals, AmbientLight, BufferGeometry, CylinderGeometry, Euler,
    IcosahedronGeometry, Mesh, Object3D, ObjectId, PerspectiveCamera, PointLight, Scene,
    StandardMaterial, TorusGeometry, Vector3,
};

use crate::stage3dscene::{color, Palette};

/// The camera's distance from the plane z = 0 and its vertical field of view.
const EYE: f32 = 9.0;
const FOV_DEG: f32 = 40.0;
/// The halo's radius at full size.
const HALO_R: f32 = 3.4;

/// One drifting solid: where it sits as a fraction of the frustum's half
/// extents at its depth, its size, and how it moves.
#[derive(Clone, Copy)]
pub(crate) struct Seed {
    /// Fractions of the half-width and half-height at `z` (−1..1 is in frame).
    pub fx: f32,
    pub fy: f32,
    pub z: f32,
    pub size: f32,
    /// Whole turns about its own axis per turn of the clock.
    pub spin: f32,
    /// Where in its bob and tumble it starts, in turns.
    pub off: f32,
    pub gem: bool,
}

const fn s(fx: f32, fy: f32, z: f32, size: f32, spin: f32, off: f32, gem: bool) -> Seed {
    Seed {
        fx,
        fy,
        z,
        size,
        spin,
        off,
        gem,
    }
}

/// Three near, three mid, three far, and two gems between: the right two
/// thirds hold the most, the left edge, where a window's text starts, the
/// fewest, and the far ones are the largest so the depth reads.
pub(crate) const SEEDS: [Seed; 11] = [
    s(0.74, -0.52, 2.0, 0.42, 1.0, 0.00, false),
    s(0.28, 0.58, 2.0, 0.34, -1.0, 0.31, false),
    s(-0.30, -0.66, 2.0, 0.30, 1.0, 0.62, false),
    s(0.52, 0.16, -1.5, 0.62, -1.0, 0.17, false),
    s(0.02, -0.18, -1.5, 0.50, 2.0, 0.48, false),
    s(0.86, 0.68, -1.5, 0.46, 1.0, 0.79, false),
    s(0.36, -0.72, -5.0, 0.85, -1.0, 0.09, false),
    s(-0.42, 0.44, -5.0, 0.78, 1.0, 0.40, false),
    s(0.78, -0.06, -5.0, 0.82, -2.0, 0.71, false),
    s(0.62, -0.12, 0.5, 0.22, 2.0, 0.25, true),
    s(-0.06, 0.30, 0.0, 0.18, -2.0, 0.55, true),
];

/// Faceted, not smooth: three radial segments make a triangular prism, and
/// unshared vertices give each face its own normal, so it catches the light
/// face by face as glass does.
fn faceted(g: BufferGeometry) -> BufferGeometry {
    let mut g = g.to_non_indexed();
    compute_vertex_normals(&mut g);
    g
}

/// The prism scene and the handles that move.
pub(crate) struct Prisms {
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub palette: Palette,
    solids: Vec<ObjectId>,
    halo: ObjectId,
    aspect: f32,
}

impl Prisms {
    pub fn new(p: Palette, aspect: f32) -> Self {
        let mut scene = Scene::new();
        // Nothing drawn is nothing: the page (and the desktop) shows.
        scene.background = color((0, 0, 0));
        scene.background_alpha = 0.0;
        // Linear fog in the page's colour: the far prisms sink into the page.
        scene.fog.color = color(p.page);
        (scene.fog.near, scene.fog.far, scene.fog.mode) = (EYE - 2.0, EYE + 9.0, 1);
        let tints = [p.key, p.fill, p.ring.0, p.ring.1];
        let solids = SEEDS
            .iter()
            .enumerate()
            .map(|(i, sd)| {
                let tint = color(tints[i % tints.len()]);
                let glass = StandardMaterial::new(tint)
                    .with_metalness(0.15)
                    .with_roughness(0.06)
                    .with_emissive(tint, if sd.gem { 0.7 } else { 0.22 });
                let geo = if sd.gem {
                    IcosahedronGeometry::new(sd.size, 0)
                } else {
                    faceted(CylinderGeometry::new(
                        sd.size,
                        sd.size,
                        sd.size * 2.6,
                        3,
                        1,
                        false,
                        0.0,
                        TAU,
                    ))
                };
                scene.add(Object3D::mesh(Mesh::new(geo, glass.into())))
            })
            .collect();
        let ring = color(p.ring.1);
        let halo_mat = StandardMaterial::new(ring)
            .with_roughness(0.4)
            .with_emissive(ring, 0.9);
        let halo = scene.add(Object3D::mesh(Mesh::new(
            TorusGeometry::new(HALO_R, 0.03, 8, 160, TAU),
            halo_mat.into(),
        )));
        scene.add_light(AmbientLight::new(color(p.page), 0.8));
        let mut key = Object3D::light(PointLight::new(color(p.key), 90.0));
        key.position = Vector3::new(-4.0, 4.5, 6.0);
        scene.add(key);
        let mut rim = Object3D::light(PointLight::new(color(p.fill), 45.0));
        rim.position = Vector3::new(5.0, -3.0, -2.0);
        scene.add(rim);
        let mut camera = PerspectiveCamera::new(FOV_DEG, aspect, 0.1, 100.0);
        camera.position = Vector3::new(0.0, 0.0, EYE);
        camera.look_at(Vector3::ZERO);
        let mut b = Self {
            scene,
            camera,
            palette: p,
            solids,
            halo,
            aspect,
        };
        b.pose(0.0);
        b
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.camera.set_aspect(aspect);
    }

    /// Where a seed sits at `phase`: its fraction of the frustum at its
    /// depth, bobbing a little on its own offset.
    pub(crate) fn place(&self, sd: &Seed, phase: f32) -> Vector3 {
        let half_h = (EYE - sd.z) * (FOV_DEG.to_radians() / 2.0).tan();
        let half_w = half_h * self.aspect;
        let bob = ((phase + sd.off) * TAU).sin() * 0.06;
        Vector3::new(sd.fx * half_w, (sd.fy + bob) * half_h, sd.z)
    }

    /// Where everything is at `phase` turns of the stage's clock.
    pub fn pose(&mut self, phase: f32) {
        let a = phase * TAU;
        for (sd, id) in SEEDS.iter().zip(self.solids.clone()) {
            let at = self.place(sd, phase);
            let o = sd.off * TAU;
            let q = Euler::new(
                0.5 + 0.3 * (a + o).sin(),
                sd.spin * a + o,
                0.35 + 0.2 * o.cos(),
            )
            .to_quaternion();
            if let Some(m) = self.scene.get_mut(id) {
                m.position = at;
                m.quaternion = q;
            }
        }
        // The halo: an ellipse low behind the prisms, rocking gently, as wide
        // as the window lets it be (a narrow one shrinks it to stay in frame).
        let half_h = (EYE + 3.0) * (FOV_DEG.to_radians() / 2.0).tan();
        let half_w = half_h * self.aspect;
        let width = (0.5 * half_w / HALO_R).min(1.0);
        if let Some(h) = self.scene.get_mut(self.halo) {
            h.position = Vector3::new(0.4 * half_w, -0.3 * half_h, -3.0);
            h.scale = Vector3::new(width, width, width);
            h.quaternion =
                Euler::new(1.2 + 0.08 * a.sin(), 0.0, 0.2 + 0.06 * a.cos()).to_quaternion();
        }
    }

    #[cfg(test)]
    pub(crate) fn solid(&self, i: usize) -> &Object3D {
        self.scene.get(self.solids[i]).expect("a solid")
    }
}

#[cfg(test)]
#[path = "stage3dprism_tests.rs"]
mod tests;
