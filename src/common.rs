#[allow(unused_imports)]
use crate::errors::Error;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct PixelLoc {
    pub i: i32,
    pub j: i32,
}

#[derive(Clone, Copy)]
pub struct RectangularArray {
    pub width: u32,
    pub height: u32,
}

impl RectangularArray {
    pub fn is_valid(&self, loc: PixelLoc) -> bool {
        (loc.i >= 0)
            && (loc.j >= 0)
            && (loc.i < self.width as i32)
            && (loc.j < self.height as i32)
    }

    pub fn get_index(&self, loc: PixelLoc) -> Option<usize> {
        if self.is_valid(loc) {
            Some((loc.j as usize) * (self.width as usize) + (loc.i as usize))
        } else {
            None
        }
    }

    pub fn get_random_loc(&self) -> PixelLoc {
        PixelLoc {
            i: (self.width as f32 * rand::random::<f32>()) as i32,
            j: (self.height as f32 * rand::random::<f32>()) as i32,
        }
    }

    #[cfg(test)]
    fn get_loc(&self, index: usize) -> Option<PixelLoc> {
        if index < self.len() {
            Some(PixelLoc {
                i: (index % (self.width as usize)) as i32,
                j: (index / (self.width as usize)) as i32,
            })
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_index_bounds() -> Result<(), Error> {
        let size = RectangularArray {
            width: 5,
            height: 10,
        };
        assert!(size.is_valid(PixelLoc { i: 2, j: 3 }));
        assert!(size.is_valid(PixelLoc { i: 4, j: 9 }));
        assert!(size.is_valid(PixelLoc { i: 0, j: 0 }));

        assert!(!size.is_valid(PixelLoc { i: 5, j: 3 }));
        assert!(!size.is_valid(PixelLoc { i: 2, j: 10 }));
        assert!(!size.is_valid(PixelLoc { i: 2, j: 15 }));
        assert!(!size.is_valid(PixelLoc { i: -1, j: 3 }));
        assert!(!size.is_valid(PixelLoc { i: 5, j: -1 }));
        assert!(!size.is_valid(PixelLoc { i: 2, j: -1 }));
        assert!(!size.is_valid(PixelLoc { i: -1, j: -1 }));

        Ok(())
    }

    #[test]
    fn test_index_lookup() -> Result<(), Error> {
        let size = RectangularArray {
            width: 5,
            height: 10,
        };

        assert_eq!(size.get_index(PixelLoc { i: 0, j: 0 }), Some(0));
        assert_eq!(size.get_index(PixelLoc { i: 1, j: 0 }), Some(1));
        assert_eq!(size.get_index(PixelLoc { i: 0, j: 1 }), Some(5));
        assert_eq!(size.get_index(PixelLoc { i: 1, j: 1 }), Some(6));
        assert_eq!(size.get_index(PixelLoc { i: 4, j: 9 }), Some(49));

        assert_eq!(size.get_index(PixelLoc { i: -1, j: 1 }), None);
        assert_eq!(size.get_index(PixelLoc { i: 4, j: 10 }), None);

        assert_eq!(size.get_loc(0), Some(PixelLoc { i: 0, j: 0 }));
        assert_eq!(size.get_loc(11), Some(PixelLoc { i: 1, j: 2 }));
        assert_eq!(size.get_loc(1), Some(PixelLoc { i: 1, j: 0 }));

        assert_eq!(size.get_loc(50), None);
        assert_eq!(size.get_loc(500000), None);

        Ok(())
    }
}
