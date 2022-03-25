use glm::RealNumber;
use nalgebra_glm as glm;
use nalgebra_glm::{vec1, TVec};

mod interpolators;

use interpolators::Interpolator;
pub use interpolators::{ExponentialSmoothing, LinearInterpolation};

// TODO: Is there a way to get make this generic cleaner, getting rid of R?
pub struct InterpolatedVector<T: RealNumber, const R: usize, I: Interpolator<T, R>> {
    current: TVec<T, R>,
    target: TVec<T, R>,
    interpolator: I,
}

impl<T: RealNumber, const R: usize, I: Interpolator<T, R>> InterpolatedVector<T, R, I> {
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

pub struct InterpolatedScalar<T: RealNumber, I: Interpolator<T, 1>>(InterpolatedVector<T, 1, I>);

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
