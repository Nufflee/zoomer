use crate::interpolation::{ExponentialSmoothing, InterpolatedScalar};

pub struct Highlighter {
    radius: InterpolatedScalar<f32, ExponentialSmoothing<f32>>,
    is_enabled: bool,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            radius: InterpolatedScalar::new(50.0, ExponentialSmoothing::new(0.25, 1.5)),
            is_enabled: false,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.radius.update(dt);
    }

    pub fn set_radius(&mut self, new_radius: f32) {
        self.radius.set_target(new_radius.max(1.0));
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.is_enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    pub fn radius(&self) -> f32 {
        if self.is_enabled {
            self.radius.current()
        } else {
            f32::INFINITY
        }
    }
}
