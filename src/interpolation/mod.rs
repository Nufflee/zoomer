use glm::RealNumber;
use nalgebra_glm as glm;
use nalgebra_glm::{vec1, TVec};

mod interpolators;

use interpolators::Interpolator;
pub use interpolators::{ExponentialSmoothing, LinearInterpolation};

pub struct InterpolatedVector<T: RealNumber, I: Interpolator<T, R>, const R: usize> {
    current: TVec<T, R>,
    target: TVec<T, R>,
    interpolator: I,
}

// TODO: Is there a way to get rid of the R parameter?
impl<T: RealNumber, I: Interpolator<T, R>, const R: usize> InterpolatedVector<T, I, R> {
    pub fn new(initial: TVec<T, R>, interpolator: I) -> Self {
        Self {
            current: initial,
            target: initial,
            interpolator,
        }
    }

    pub fn new_zeroed(interpolator: I) -> Self {
        Self::new(TVec::zeros(), interpolator)
    }

    pub fn set_target(&mut self, target: TVec<T, R>) {
        self.target = target;
    }

    pub fn update(&mut self, dt: T) -> TVec<T, R> {
        self.current = self.interpolator.interpolate(self.current, self.target, dt);

        self.current
    }

    pub fn current(&self) -> TVec<T, R> {
        self.current
    }

    pub fn target(&self) -> TVec<T, R> {
        self.target
    }
}

pub struct InterpolatedScalar<T: RealNumber, I: Interpolator<T, 1>>(InterpolatedVector<T, I, 1>);

impl<T: RealNumber, I: Interpolator<T, 1>> InterpolatedScalar<T, I> {
    pub fn new(initial: T, interpolator: I) -> Self {
        Self(InterpolatedVector::new(vec1(initial), interpolator))
    }

    pub fn new_zeroed(interpolator: I) -> Self {
        Self::new(T::zero(), interpolator)
    }

    pub fn set_target(&mut self, target: T) {
        self.0.set_target(vec1(target));
    }

    pub fn update(&mut self, dt: T) {
        self.0.update(dt);
    }

    pub fn current(&self) -> T {
        self.0.current().x
    }

    pub fn target(&self) -> T {
        self.0.target().x
    }
}
