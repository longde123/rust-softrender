//! Depth Buffer attachment definition

use num_traits::{NumCast, Bounded};

use ::numeric::FloatScalar;

/// Defines a depth buffer attachment.
///
/// This is automatically implemented for type that satisfy the dependent traits
pub trait Depth: super::Attachment + PartialOrd {
    /// The value that represents the farthest away depth value.
    fn far() -> Self;

    /// Create the depth value from some scalar value, as derived from the vertex data.
    fn from_scalar<N: FloatScalar>(n: N) -> Self;
}

impl Depth for () {
    #[inline(always)]
    fn far() -> () { () }

    #[inline(always)]
    fn from_scalar<N: FloatScalar>(_: N) -> () { () }
}

macro_rules! impl_depth_primitives {
    ($($t:ty),+) => {
        $(
            impl Depth for $t {
                #[inline(always)]
                fn far() -> $t { <$t as Bounded>::min_value() }

                #[inline(always)]
                fn from_scalar<N: FloatScalar>(n: N) -> $t {
                    <$t as NumCast>::from(n).expect("Invalid Cast")
                }
            }
        )+
    }
}

impl_depth_primitives!(i8, i16, i32, i64, u8, u16, u32, u64, isize, usize, f32, f64);