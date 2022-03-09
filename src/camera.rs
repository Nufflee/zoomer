use std::ops::RangeInclusive;

use nalgebra_glm::{clamp_vec, vec2, vec2_to_vec3, Mat4, Vec2};

/// A 2D camera.
pub struct Camera {
    /// The translation of the camera in camera space.
    translation: Vec2,
    pub(crate) zoom_factor: f32,
    zoom_range: RangeInclusive<f32>,
    translation_range: Vec2,
}

impl Camera {
    /// Creates a new camera.
    /// - `zoom_range`: the min and max value of the zoom_factor
    /// - `translation_range`: the symmetric range of the translation in world space (`±translation_range.x` on x axis and `±translation_range.y` on y axis)
    pub fn new(zoom_range: RangeInclusive<f32>, translation_range: Vec2) -> Self {
        Self {
            translation: Vec2::zeros(),
            zoom_factor: 1.0,
            zoom_range,
            translation_range,
        }
    }

    pub fn translate(&mut self, translation: Vec2) {
        self.translation += translation;

        self.translation = clamp_vec(
            &self.position(),
            &vec2(-self.translation_range.x, -self.translation_range.y),
            &vec2(self.translation_range.x, self.translation_range.y),
        ) * self.zoom_factor
    }

    pub fn zoom(&mut self, zoom_multiplier: f32, point: Vec2) {
        let new_zoom_factor = (self.zoom_factor * zoom_multiplier)
            .clamp(*self.zoom_range.start(), *self.zoom_range.end());

        // Recompute the zoom multiplier as it may have changed due to the clamp.
        let zoom_multiplier = new_zoom_factor / self.zoom_factor;

        self.zoom_factor = new_zoom_factor;
        self.translate(point - point * zoom_multiplier);
    }

    /// Converts from screen space coordinates or NDC ([-1, 1] x [-1, 1]) to camera space coordinates ([`-self.zoom_factor`, `self.zoom_factor`] x [`-self.zoom_factor`, `self.zoom_factor`]).
    pub fn screen_to_camera_space(&self, screen_coords: Vec2) -> Vec2 {
        screen_coords - self.translation
    }

    /// Converts from screen space coordintes or NDC ([-1, 1] x [-1, 1]) to global world space coordinates
    pub fn screen_to_world_space(&self, screen_coords: Vec2) -> Vec2 {
        self.screen_to_camera_space(screen_coords) / self.zoom_factor
    }

    /// Converts the camera's transformations into an equivalent homogenous matrix.
    pub fn to_homogenous(&self) -> Mat4 {
        Mat4::new_translation(&vec2_to_vec3(&self.translation))
            * Mat4::new_scaling(self.zoom_factor)
    }

    /// Returns the camera's position in world space.
    pub fn position(&self) -> Vec2 {
        self.translation / self.zoom_factor
    }
}
