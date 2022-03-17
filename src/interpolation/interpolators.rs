use nalgebra_glm as glm;
use nalgebra_glm::{RealNumber, TVec};

pub trait Interpolator<T: RealNumber, const R: usize> {
    fn interpolate(&mut self, current: TVec<T, R>, target: TVec<T, R>, dt: T) -> TVec<T, R>;
}

pub struct ExponentialSmoothing<T> {
    length_sec: T,
    exp_rate: T,
}

impl<T> ExponentialSmoothing<T> {
    /// Create a new exponential smoothing interpolator (https://en.wikipedia.org/wiki/Exponential_smoothing, http://www.viniciusgraciano.com/blog/exponential-smoothing/).
    /// - `length_sec`: The length of the interpolation in seconds.
    /// - `exp_rate`: The exponential rate of smoothing (10^x).
    pub fn new(length_sec: T, exp_rate: T) -> Self {
        Self {
            length_sec,
            exp_rate,
        }
    }
}

impl<T: RealNumber, const R: usize> Interpolator<T, R> for ExponentialSmoothing<T> {
    fn interpolate(&mut self, current: TVec<T, R>, target: TVec<T, R>, dt: T) -> TVec<T, R> {
        glm::lerp(
            &current,
            &target,
            T::one()
                - (T::one() / T::from_f32(10.0).unwrap().powf(self.exp_rate))
                    .powf(dt / self.length_sec),
        )
    }
}

pub struct LinearInterpolation<T> {
    k: T,
    time: T,
}

impl<T: RealNumber> LinearInterpolation<T> {
    fn new(length_sec: T) -> Self {
        Self {
            k: T::one() / length_sec,
            time: T::zero(),
        }
    }
}

impl<T: RealNumber, const R: usize> Interpolator<T, R> for LinearInterpolation<T> {
    fn interpolate(&mut self, current: TVec<T, R>, target: TVec<T, R>, dt: T) -> TVec<T, R> {
        self.time += dt;

        glm::lerp(&current, &target, self.k * self.time)
    }
}

#[cfg(test)]
mod tests {
    use glm::vec2;

    use crate::interpolation::{InterpolatedScalar, InterpolatedVector};

    use super::*;

    #[test]
    fn linear_interpolation() {
        let mut var = InterpolatedScalar::new_zeroed(LinearInterpolation::new(1.0_f32));
        var.set_target(10.0);

        var.update(0.5);

        assert_eq!(var.current(), 5.0);
    }

    #[test]
    fn linear_interpolation_10_seconds() {
        let mut var = InterpolatedScalar::new_zeroed(LinearInterpolation::new(10.0_f32));
        var.set_target(10.0);

        var.update(5.0);

        assert_eq!(var.current(), 5.0);
    }

    #[test]
    fn linear_interpolation_vec() {
        let mut var = InterpolatedVector::new_zeroed(LinearInterpolation::new(1.0_f32));
        var.set_target(vec2(10.0, 5.0));

        var.update(0.5);

        assert_eq!(var.current(), vec2(5.0, 2.5));
    }

    #[test]
    fn exponential_smoothing_with_initial_value() {
        let mut var = InterpolatedVector::new(vec2(1.0, 2.0), ExponentialSmoothing::new(1.0, 5.0));
        var.set_target(vec2(10.0, 5.0));

        var.update(1.0);

        assert!((var.current() - vec2(10.0, 5.0)).abs() < vec2(1e-3, 1e-3));
    }
}
