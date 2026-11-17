//! `mercury`, the scene the liquid-metal theme draws behind its panes
//! (`stage3d`): pools of mercury at three depths, each a few chrome lobes
//! circling one another so close that they read as one blob forever folding
//! into itself, with lone droplets between, every lobe wobbling as liquid
//! does, in a fog of the page's own colour.
//!
//! What makes it metal and not grey plastic is what it reflects. A fully
//! metallic surface lit by point lights alone is black with pinprick
//! highlights; chrome is a picture of the room. So the scene carries a
//! small studio of its own ([`crate::stage3dstudio`]): a cube environment, drawn here,
//! of a dark floor in the page's colour, a softbox overhead and three tall
//! strip lights (two in the key's cool steel, one in the activity's warm
//! copper), which threers prefilters and every lobe mirrors.
//!
//! Like `prism`, each pool is placed at a fraction of the frustum's half
//! extents at its depth, so a tall or an ultrawide window keeps it in frame,
//! and every motion is a whole number of turns per turn of the clock, so the
//! picture at phase 1 is the picture at phase 0 and the wrap never jumps.
use std::f32::consts::TAU;
use std::sync::Arc;

use threers::cameras::Camera as _;
use threers::{
    AmbientLight, Mesh, Object3D, ObjectId, PerspectiveCamera, PointLight, Scene, SphereGeometry,
    StandardMaterial, Vector3,
};

use crate::stage3dscene::{color, Palette};
use crate::stage3dstudio::studio;

/// The camera's distance from the plane z = 0 and its vertical field of view.
const EYE: f32 = 9.0;
const FOV_DEG: f32 = 40.0;

/// One pool of mercury: where it sits as a fraction of the frustum's half
/// extents at its depth, its size, how many lobes it has and how they move.
#[derive(Clone, Copy)]
pub(crate) struct Pool {
    /// Fractions of the half-width and half-height at `z` (−1..1 is in frame).
    pub fx: f32,
    pub fy: f32,
    pub z: f32,
    pub size: f32,
    /// Lobes circling the pool's centre; 1 is a lone droplet.
    pub lobes: usize,
    /// Whole turns the lobes go round the centre per turn of the clock.
    pub spin: f32,
    /// Where in its circling and wobble it starts, in turns.
    pub off: f32,
}

const fn p(fx: f32, fy: f32, z: f32, size: f32, lobes: usize, spin: f32, off: f32) -> Pool {
    Pool {
        fx,
        fy,
        z,
        size,
        lobes,
        spin,
        off,
    }
}

/// Two near pools and two lone drops, two mid, two far: the right two
/// thirds hold the most, the left edge, where a window's text starts, the
/// fewest, and the far ones are the largest so the depth reads.
pub(crate) const POOLS: [Pool; 8] = [
    p(0.60, -0.48, 2.0, 0.58, 3, 1.0, 0.00),
    p(0.18, 0.58, 2.0, 0.40, 2, -1.0, 0.37),
    p(0.46, 0.10, -1.5, 0.80, 3, -1.0, 0.18),
    p(-0.32, -0.52, -1.5, 0.62, 2, 1.0, 0.66),
    p(0.80, 0.62, -5.0, 1.15, 2, 1.0, 0.09),
    p(-0.46, 0.40, -5.0, 1.10, 3, -1.0, 0.52),
    p(0.04, -0.10, 0.5, 0.20, 1, 2.0, 0.25),
    p(0.88, -0.02, 0.0, 0.17, 1, -2.0, 0.81),
];

/// How far a lobe's centre sits from its pool's, in pool sizes: under the
/// lobes' own radii, so neighbours always overlap and the pool reads whole.
const REACH: f32 = 0.3;
/// How far a lobe squashes and stretches as it wobbles.
const WOBBLE: f32 = 0.11;

/// The radius of lobe `i` of a pool of `size`: the first the largest.
fn lobe_radius(size: f32, i: usize) -> f32 {
    size * (0.66 - 0.07 * i as f32)
}

/// The mercury scene and the handles that move.
pub(crate) struct Mercury {
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub palette: Palette,
    /// Each pool's lobes, in [`POOLS`] order.
    lobes: Vec<Vec<ObjectId>>,
    aspect: f32,
}

impl Mercury {
    pub fn new(p: Palette, aspect: f32) -> Self {
        let mut scene = Scene::new();
        // Nothing drawn is nothing: the page (and the desktop) shows.
        scene.background = color((0, 0, 0));
        scene.background_alpha = 0.0;
        scene.fog.color = color(p.page);
        (scene.fog.near, scene.fog.far, scene.fog.mode) = (EYE - 2.0, EYE + 9.0, 1);
        scene.environment = Some(Arc::new(studio(&p)));
        scene.environment_intensity = 1.0;
        // Chrome is near-white metal; the key gives it the theme's cast.
        let tint = color({
            let (r, g, b) = p.key;
            let up = |c: u8| (u16::from(c) + 2 * 235) as f32 / 3.0;
            (up(r) as u8, up(g) as u8, up(b) as u8)
        });
        let chrome = StandardMaterial::new(tint)
            .with_metalness(1.0)
            .with_roughness(0.1);
        let lobes = POOLS
            .iter()
            .map(|pool| {
                (0..pool.lobes)
                    .map(|_| {
                        scene.add(Object3D::mesh(Mesh::new(
                            SphereGeometry::new(1.0, 48, 32),
                            chrome.clone().into(),
                        )))
                    })
                    .collect()
            })
            .collect();
        scene.add_light(AmbientLight::new(color(p.page), 0.5));
        let mut key = Object3D::light(PointLight::new(color(p.key), 70.0));
        key.position = Vector3::new(-4.0, 4.5, 6.0);
        scene.add(key);
        let mut rim = Object3D::light(PointLight::new(color(p.fill), 45.0));
        rim.position = Vector3::new(5.0, -2.5, 1.0);
        scene.add(rim);
        let mut camera = PerspectiveCamera::new(FOV_DEG, aspect, 0.1, 100.0);
        camera.position = Vector3::new(0.0, 0.0, EYE);
        camera.look_at(Vector3::ZERO);
        let mut b = Self {
            scene,
            camera,
            palette: p,
            lobes,
            aspect,
        };
        b.pose(0.0);
        b
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.camera.set_aspect(aspect);
    }

    /// Where a pool's centre sits at `phase`: its fraction of the frustum at
    /// its depth, bobbing a little on its own offset.
    pub(crate) fn centre(&self, pool: &Pool, phase: f32) -> Vector3 {
        let half_h = (EYE - pool.z) * (FOV_DEG.to_radians() / 2.0).tan();
        let half_w = half_h * self.aspect;
        let bob = ((phase + pool.off) * TAU).sin() * 0.05;
        Vector3::new(pool.fx * half_w, (pool.fy + bob) * half_h, pool.z)
    }

    /// Where everything is at `phase` turns of the stage's clock.
    pub fn pose(&mut self, phase: f32) {
        let a = phase * TAU;
        for (pool, ids) in POOLS.iter().zip(self.lobes.clone()) {
            let centre = self.centre(pool, phase);
            let o = pool.off * TAU;
            let n = ids.len();
            for (i, id) in ids.into_iter().enumerate() {
                // Lone drops sit still and only wobble; lobes circle the
                // centre on a tilted ellipse, so they pass before and behind
                // one another, and the pool turns over as it goes round.
                let at = if n == 1 {
                    Vector3::ZERO
                } else {
                    let t = pool.spin * a + o + TAU * i as f32 / n as f32;
                    let reach = pool.size * REACH * (1.0 + 0.25 * (2.0 * a + o).sin());
                    Vector3::new(t.cos(), 0.45 * t.sin(), 0.6 * t.sin()) * reach
                };
                let r = lobe_radius(pool.size, i);
                let w = WOBBLE * (3.0 * a + o + 1.7 * i as f32).sin();
                if let Some(m) = self.scene.get_mut(id) {
                    m.position = centre + at;
                    m.scale = Vector3::new(r * (1.0 + w), r * (1.0 - w), r * (1.0 + 0.5 * w));
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn lobe(&self, pool: usize, i: usize) -> &Object3D {
        self.scene.get(self.lobes[pool][i]).expect("a lobe")
    }
}

#[cfg(test)]
#[path = "stage3dmercury_tests.rs"]
mod tests;
