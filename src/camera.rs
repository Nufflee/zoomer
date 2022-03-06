use nalgebra_glm::{vec2_to_vec3, Mat4, Vec2};

/// A 2D camera.
pub struct Camera {
    translation: Vec2,
    pub zoom_factor: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            translation: Vec2::zeros(),
            zoom_factor: 1.0,
        }
    }
}

impl Camera {
    pub fn translate(&mut self, translation: Vec2) {
        self.translation += translation;
    }

    pub fn translate_to(&mut self, translation: Vec2) {
        self.translation = translation;
    }

    pub fn zoom(&mut self, zoom_factor: f32, point: Vec2) {
        self.zoom_factor *= zoom_factor;
        self.translate(point - point * zoom_factor);
    }

    /// Converts from normalized screen space coordinates or NDC ([-1, 1] x [-1, 1]) to world space coordinates.
    pub fn screen_to_world_space_coords(&self, screen_coords: Vec2) -> Vec2 {
        screen_coords - self.translation
    }

    /// Converts the camera's transformations into an equivalent homogenous matrix.
    pub fn to_homogenous(&self) -> Mat4 {
        Mat4::new_translation(&vec2_to_vec3(&self.translation))
            * Mat4::new_scaling(self.zoom_factor)
    }
}
