use std::ops::{Add, AddAssign};

use nalgebra::Vector2;
use nalgebra::coordinates::XY;

use super::Dimensions;

/// Simple 2D Coordinate structure. Easily converts to/from nalgebra's `Vector2D<u32>` for more complex operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd)]
pub struct Coordinate {
    /// x-coordinate
    pub x: u32,
    /// y-coordinate
    pub y: u32,
}

impl Default for Coordinate {
    fn default() -> Coordinate {
        Coordinate::new(0, 0)
    }
}

impl Coordinate {
    /// Create new `Coordinate` from `x` and `y` components
    #[inline]
    pub fn new(x: u32, y: u32) -> Coordinate {
        Coordinate { x, y }
    }

    /// Construct a `Coordinate` from a 2D Vector
    #[inline]
    pub fn from_vector(v: Vector2<u32>) -> Coordinate {
        let XY { x, y } = *v;
        Coordinate::new(x, y)
    }

    /// Convert the `Coordinate` to a 2D Veector.
    #[inline]
    pub fn into_vector(self) -> Vector2<u32> {
        let Coordinate { x, y } = self;

        Vector2::new(x, y)
    }

    /// Convert a 2D coordinate into a 1D array index using the given `Dimensions`
    #[inline]
    pub fn into_index(self, dimensions: Dimensions) -> usize {
        debug_assert!(self.x < dimensions.width);
        debug_assert!(self.y < dimensions.height);

        self.x as usize + self.y as usize * dimensions.width as usize
    }

    /// Convert a 1D array index into a 2D coordinate using the given `Dimensions`
    #[inline]
    pub fn from_index(index: usize, dimensions: Dimensions) -> Coordinate {
        let width = dimensions.width as usize;

        let x = index % width;
        let y = (index - x) / width;

        Coordinate { x: x as u32, y: y as u32 }
    }
}

#[cfg(test)]
mod test {
    use super::{Dimensions, Coordinate};

    #[test]
    fn coordinate_index() {
        let dim = Dimensions::new(10, 20);

        let mut i = 0;

        for y in 0..dim.height {
            for x in 0..dim.width {
                let coord = Coordinate::new(x, y);

                assert_eq!(coord.into_index(dim), i);
                assert_eq!(Coordinate::from_index(i, dim), coord);

                i += 1;
            }
        }
    }
}

impl From<Vector2<u32>> for Coordinate {
    #[inline(always)]
    fn from(v: Vector2<u32>) -> Coordinate {
        Coordinate::from_vector(v)
    }
}

impl From<Coordinate> for Vector2<u32> {
    #[inline(always)]
    fn from(coord: Coordinate) -> Vector2<u32> {
        coord.into_vector()
    }
}

impl Add for Coordinate {
    type Output = Coordinate;

    fn add(self, rhs: Coordinate) -> Coordinate {
        Coordinate {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Coordinate {
    fn add_assign(&mut self, rhs: Coordinate) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}