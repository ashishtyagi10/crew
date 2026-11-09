//! What the 3D backdrop draws (`stage3d`): a threers scene, lit in the live
//! theme's colours, and how it moves on the stage's clock.
//!
//! `orbit`: a slowly turning metal torus knot low on the right, a ring of
//! small crystals going round it, a key light in the theme's accent and a
//! fill in its activity colour, over an ambient wash of the page. It sits
//! where a window has the least text (right, low), and stays dim: the panes'
//! words are what the window is for. Like `prism` it is placed by the
//! frustum, not in world units: its centre at a fraction of the half-width,
//! so an ultrawide window keeps it right of the text, and narrower than
//! [`FULL_ASPECT`] the whole orbit shrinks with the window, so a tall one
//! keeps it in frame rather than past the right edge.
//!
//! Every motion is a whole number of turns per turn of the clock, so the
//! picture at phase 1 is the picture at phase 0 and the wrap never jumps.
use std::f32::consts::TAU;

use threers::cameras::Camera as _;
use threers::{
    AmbientLight, Color, Euler, IcosahedronGeometry, Mesh, Object3D, ObjectId, PerspectiveCamera,
    PointLight, Scene, StandardMaterial, TorusKnotGeometry, Vector3,
};

/// Crystals in the ring.
const CRYSTALS: usize = 7;
/// The camera's distance from the plane z = 0 and its vertical field of view.
const EYE: f32 = 8.0;
const FOV_DEG: f32 = 40.0;
/// Where the orbit's centre sits, as fractions of the frustum's half extents
/// at z = 0: the right half, low.
const AT_X: f32 = 0.5;
const AT_Y: f32 = -0.275;
/// The widest aspect at which the orbit is drawn at full size: the 16:10
/// window it was designed on. Narrower, it scales down by the width.
const FULL_ASPECT: f32 = 1.6;

/// The colours a scene is lit in, taken from a theme.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Palette {
    pub key: (u8, u8, u8),
    pub fill: (u8, u8, u8),
    pub page: (u8, u8, u8),
    /// The gradient ring's two poles, or the key and fill on a theme with
    /// no ring: `prism` tints its solids and its halo with them.
    pub ring: ((u8, u8, u8), (u8, u8, u8)),
    pub dark: bool,
}

impl Palette {
    /// The theme's colours with its own accent as the key light.
    #[cfg(test)]
    pub fn of(t: &crew_theme::Theme) -> Self {
        Self::lit(t, t.accent_default)
    }

    /// The theme's colours with `key` as the key light: the accent as the
    /// app draws it (the user's `accent` setting, floored for the page), so
    /// the knot is lit in the colour the ring, the nav and the cursor wear.
    pub fn lit(t: &crew_theme::Theme, key: (u8, u8, u8)) -> Self {
        Self {
            key,
            fill: t.activity,
            page: t.page_bg,
            ring: t.modern.map_or((key, t.activity), |m| (m.pole_a, m.pole_b)),
            dark: t.dark,
        }
    }
}

pub(crate) fn color((r, g, b): (u8, u8, u8)) -> Color {
    let lin = |c: u8| {
        let c = f32::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let mut out = Color::from_hex(0);
    (out.r, out.g, out.b) = (lin(r), lin(g), lin(b));
    out
}

/// A built scene and the handles that move.
pub(crate) struct Built {
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub palette: Palette,
    knot: ObjectId,
    crystals: Vec<ObjectId>,
    aspect: f32,
}

impl Built {
    pub fn orbit(p: Palette, aspect: f32) -> Self {
        let mut scene = Scene::new();
        // Nothing drawn is nothing: premultiplied black at zero alpha, so
        // the page (and, on a see-through window, the desktop) shows.
        scene.background = Color::from_hex(0);
        scene.background_alpha = 0.0;
        let metal = StandardMaterial::new(color(p.key))
            .with_metalness(0.85)
            .with_roughness(0.28);
        let knot = scene.add(Object3D::mesh(Mesh::new(
            TorusKnotGeometry::new(0.7, 0.22, 220, 28, 2, 3),
            metal.into(),
        )));
        let gem = StandardMaterial::new(color(p.fill))
            .with_metalness(0.2)
            .with_roughness(0.15)
            .with_emissive(color(p.fill), 0.8);
        let crystals = (0..CRYSTALS)
            .map(|_| {
                scene.add(Object3D::mesh(Mesh::new(
                    IcosahedronGeometry::new(0.2, 0),
                    gem.clone().into(),
                )))
            })
            .collect();
        scene.add_light(AmbientLight::new(
            color(p.page),
            if p.dark { 0.6 } else { 0.9 },
        ));
        let mut key = Object3D::light(PointLight::new(color(p.key), 60.0));
        key.position = Vector3::new(-3.0, 4.0, 4.0);
        scene.add(key);
        let mut fill = Object3D::light(PointLight::new(color(p.fill), 35.0));
        fill.position = Vector3::new(4.0, -2.0, 3.0);
        scene.add(fill);
        let mut camera = PerspectiveCamera::new(FOV_DEG, aspect, 0.1, 100.0);
        camera.position = Vector3::new(0.0, 1.2, EYE);
        camera.look_at(Vector3::ZERO);
        let mut b = Self {
            scene,
            camera,
            palette: p,
            knot,
            crystals,
            aspect,
        };
        b.pose(0.0);
        b
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.camera.set_aspect(aspect);
    }

    /// The orbit's centre and size for the window's aspect: a fraction of the
    /// frustum's half extents at z = 0, and full size down to
    /// [`FULL_ASPECT`], smaller with the width below it.
    pub(crate) fn frame(&self) -> (Vector3, f32) {
        let half_h = EYE * (FOV_DEG.to_radians() / 2.0).tan();
        let half_w = half_h * self.aspect;
        let size = (self.aspect / FULL_ASPECT).min(1.0);
        (Vector3::new(AT_X * half_w, AT_Y * half_h, 0.0), size)
    }

    /// Where everything is at `phase` turns of the stage's clock.
    pub fn pose(&mut self, phase: f32) {
        let a = phase * TAU;
        let (centre, size) = self.frame();
        if let Some(k) = self.scene.get_mut(self.knot) {
            k.position = centre;
            k.scale = Vector3::new(size, size, size);
            k.quaternion = Euler::new(0.35 + 0.15 * a.sin(), a, 0.0).to_quaternion();
        }
        for (i, id) in self.crystals.iter().enumerate() {
            let at = a + i as f32 * TAU / CRYSTALS as f32;
            // Two turns of the clock spin each crystal twice on its own axis.
            let spin = Euler::new(2.0 * a, at, 0.0).to_quaternion();
            if let Some(c) = self.scene.get_mut(*id) {
                let ring = Vector3::new(1.7 * at.cos(), 0.35 * at.sin(), 1.7 * at.sin());
                c.position = centre + ring * size;
                c.scale = Vector3::new(size, size, size);
                c.quaternion = spin;
            }
        }
    }
}

#[cfg(test)]
#[path = "stage3dscene_tests.rs"]
mod tests;
