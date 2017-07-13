use nalgebra::Scalar;

pub mod interpolate;
pub mod utils;

use self::interpolate::Interpolate;

pub use num_traits::Float;

pub trait FloatScalar: Float + Scalar + Interpolate {}

impl<T> FloatScalar for T where T: Float + Scalar + Interpolate {}

