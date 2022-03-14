use std::ops::RangeInclusive;

use nalgebra_glm::{clamp_vec, lerp, lerp_scalar, vec2_to_vec3, Mat4, Vec2};

/// A 2D camera.
pub struct Camera {
    /// The position of the camera in camera space.
    position: Vec2,
    position_target: Vec2,
    pub(crate) zoom_factor: f32,
    zoom_factor_target: f32,

    zoom_range: RangeInclusive<f32>,
    /// The range of position in world space.
    position_range: Vec2,
}

impl Camera {
    /// Creates a new camera.
    /// - `zoom_range`: the min and max value of the zoom_factor
    /// - `position_range`: the symmetric range of the position in world space (`±position_range.x` on x axis and `±position_range.y` on y axis)
    pub fn new(zoom_range: RangeInclusive<f32>, position_range: Vec2) -> Self {
        Self {
            position: Vec2::zeros(),
            position_target: Vec2::zeros(),
            zoom_factor: 1.0,
            zoom_factor_target: 1.0,
            zoom_range,
            position_range,
        }
    }

    /// Smoothly translates the camera by the given `translation`.
    pub fn translate(&mut self, translation: Vec2) {
        self.position_target += translation;
    }

    pub fn clamp_me_daddy(&mut self) {
        let camera_position_range = self.world_to_camera_space(self.position_range);

        self.position_target = clamp_vec(
            &self.position_target,
            &-camera_position_range,
            &camera_position_range,
        );
    }

    /// Smoothly zooms the camera in by the given zoom factor towards the given point.
    pub fn zoom(&mut self, zoom_multiplier: f32, screen_point: Vec2) {
        let new_zoom_factor = (self.zoom_factor_target * zoom_multiplier)
            .clamp(*self.zoom_range.start(), *self.zoom_range.end());

        // Recompute the zoom multiplier as it may have changed due to the clamp.
        let zoom_multiplier = new_zoom_factor / self.zoom_factor_target;

        self.zoom_factor_target = new_zoom_factor;

        // Convert to camera space using the position target, not current position
        let point = screen_point - self.position_target;
        self.translate(point - point * zoom_multiplier);
    }

    pub fn update(&mut self, dt: f32) {
        // This type of lerp is an exponential smoothing (https://en.wikipedia.org/wiki/Exponential_smoothing, http://www.viniciusgraciano.com/blog/exponential-smoothing/)
        let t = 1.0 - (0.0001f32).powf(dt);

        self.zoom_factor = lerp_scalar(self.zoom_factor, self.zoom_factor_target, t);
        self.position = lerp(&self.position, &self.position_target, t);
    }

    /// Converts from screen space coordinates or NDC ([-1, 1] x [-1, 1]) to camera space coordinates ([`-self.zoom_factor`, `self.zoom_factor`] x [`-self.zoom_factor`, `self.zoom_factor`]).
    pub fn screen_to_camera_space(&self, screen_coords: Vec2) -> Vec2 {
        screen_coords - self.position
    }

    /// Converts from screen space coordintes or NDC ([-1, 1] x [-1, 1]) to global world space coordinates
    pub fn screen_to_world_space(&self, screen_coords: Vec2) -> Vec2 {
        self.screen_to_camera_space(screen_coords) / self.zoom_factor
    }

    /// Converts the camera's transformations into an equivalent homogenous matrix.
    pub fn to_homogenous(&self) -> Mat4 {
        Mat4::new_translation(&vec2_to_vec3(&self.position)) * Mat4::new_scaling(self.zoom_factor)
    }

    fn world_to_camera_space(&self, world_coords: Vec2) -> Vec2 {
        world_coords * self.zoom_factor
    }

    fn camera_to_world_space(&self, camera_coords: Vec2) -> Vec2 {
        camera_coords / self.zoom_factor
    }

    /// Returns the camera's position in world space.
    pub fn position(&self) -> Vec2 {
        self.camera_to_world_space(self.position)
    }
}
