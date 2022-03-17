use std::ops::RangeInclusive;

use nalgebra_glm::{clamp_vec, vec2_to_vec3, Mat4, Vec2};

use crate::interpolation::{ExponentialSmoothing, InterpolatedScalar, InterpolatedVector};

/// A 2D camera.
pub struct Camera {
    /// The position of the camera in camera space.
    position: InterpolatedVector<f32, ExponentialSmoothing<f32>, 2>,
    zoom_factor: InterpolatedScalar<f32, ExponentialSmoothing<f32>>,

    zoom_range: RangeInclusive<f32>,
    /// The range of position in world space.
    position_range: Vec2,
}

impl Camera {
    /// Creates a new camera.
    /// - `zoom_range`: the min and max value of the zoom_factor
    /// - `position_range`: the symmetric range of the position in world space (`±position_range.x` on x axis and `±position_range.y` on y axis)
    pub fn new(zoom_range: RangeInclusive<f32>, position_range: Vec2) -> Self {
        const LENGTH: f32 = 0.5;
        const RATE: f32 = 2.5;

        Self {
            position: InterpolatedVector::new_zeroed(ExponentialSmoothing::new(LENGTH, RATE)),
            zoom_factor: InterpolatedScalar::new(1.0, ExponentialSmoothing::new(LENGTH, RATE)),
            zoom_range,
            position_range,
        }
    }

    /// Smoothly translates the camera by the given `translation`.
    pub fn translate(&mut self, translation: Vec2) {
        self.position
            .set_target(self.position.target() + translation);
    }

    pub fn clamp_me_daddy(&mut self) {
        let camera_position_range = self.world_to_camera_space(self.position_range);

        self.position.set_target(clamp_vec(
            &self.position.target(),
            &-camera_position_range,
            &camera_position_range,
        ));
    }

    /// Smoothly zooms the camera in by the given zoom factor towards the given point.
    pub fn zoom(&mut self, zoom_multiplier: f32, screen_point: Vec2) {
        let new_zoom_factor = (self.zoom_factor.target() * zoom_multiplier)
            .clamp(*self.zoom_range.start(), *self.zoom_range.end());

        // Recompute the zoom multiplier as it may have changed due to the clamp.
        let zoom_multiplier = new_zoom_factor / self.zoom_factor.target();

        self.zoom_factor.set_target(new_zoom_factor);

        // Convert to camera space using the position target, not current position
        let point = screen_point - self.position.target();
        self.translate(point - point * zoom_multiplier);
    }

    pub fn update(&mut self, dt: f32) {
        self.zoom_factor.update(dt);
        self.position.update(dt);
    }

    /// Converts from screen space coordinates or NDC ([-1, 1] x [-1, 1]) to camera space coordinates ([`-self.zoom_factor`, `self.zoom_factor`] x [`-self.zoom_factor`, `self.zoom_factor`]).
    pub fn screen_to_camera_space(&self, screen_coords: Vec2) -> Vec2 {
        screen_coords - self.position.current()
    }

    /// Converts from screen space coordintes or NDC ([-1, 1] x [-1, 1]) to global world space coordinates
    pub fn screen_to_world_space(&self, screen_coords: Vec2) -> Vec2 {
        self.screen_to_camera_space(screen_coords) / self.zoom_factor.current()
    }

    /// Converts the camera's transformations into an equivalent homogenous matrix.
    pub fn to_homogenous(&self) -> Mat4 {
        Mat4::new_translation(&vec2_to_vec3(&self.position.current()))
            * Mat4::new_scaling(self.zoom_factor.current())
    }

    fn world_to_camera_space(&self, world_coords: Vec2) -> Vec2 {
        world_coords * self.zoom_factor.current()
    }

    fn camera_to_world_space(&self, camera_coords: Vec2) -> Vec2 {
        camera_coords / self.zoom_factor.current()
    }

    /// Returns the camera's position in world space.
    pub fn position(&self) -> Vec2 {
        self.camera_to_world_space(self.position.current())
    }

    pub fn zoom_factor(&self) -> f32 {
        self.zoom_factor.current()
    }
}
