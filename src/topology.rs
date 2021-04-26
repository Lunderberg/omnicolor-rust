use std::collections::HashMap;

use itertools::Itertools;

#[allow(unused_imports)]
use crate::errors::Error;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct PixelLoc {
    pub i: i32,
    pub j: i32,
}

impl PixelLoc {
    // Line between two points.  Uses Bresenham's, modified to have no
    // diagonal openings.
    pub fn line_to(&self, other: PixelLoc) -> Vec<PixelLoc> {
        if self.i == other.i {
            self.vertical_line_to(other.j)
        } else {
            self.finite_slope_line_to(other)
        }
    }

    fn finite_slope_line_to(&self, other: PixelLoc) -> Vec<PixelLoc> {
        let slope = ((other.j - self.j) as f64) / ((other.i - self.i) as f64);
        let offset = (self.j as f64) - slope * (self.i as f64);

        let mut output = Vec::new();

        let mut prev_j = None;

        let imin = self.i.min(other.i);
        let imax = self.i.max(other.i);
        for i in imin..=imax {
            let j1 = (slope * (i as f64) + offset).floor() as i32;
            let j2 = prev_j.unwrap_or(j1);

            let jmin = j1.min(j2);
            let jmax = j1.max(j2);

            for j in jmin..=jmax {
                output.push(PixelLoc { i, j });
            }

            prev_j = Some(j1);
        }

        output
    }

    fn vertical_line_to(&self, other_j: i32) -> Vec<PixelLoc> {
        let jmin = self.j.min(other_j);
        let jmax = self.j.max(other_j);
        (jmin..=jmax).map(|j| PixelLoc { i: self.i, j }).collect()
    }
}

#[derive(Clone)]
pub struct Topology {
    pub size: RectangularArray,
    pub portals: HashMap<PixelLoc, PixelLoc>,
}

// Currently, most of these just delegate to RectangularArray, but
// they'll be more differentiated once there are multiple layers to
// the image.
impl Topology {
    pub fn is_valid(&self, loc: PixelLoc) -> bool {
        self.size.is_valid(loc)
    }

    pub fn get_index(&self, loc: PixelLoc) -> Option<usize> {
        self.size.get_index(loc)
    }

    pub fn iter_adjacent(
        &self,
        loc: PixelLoc,
    ) -> impl Iterator<Item = PixelLoc> + '_ {
        let within_layer = self.size.iter_adjacent(loc);
        let by_portal = self.portals.get(&loc).into_iter().map(|x| *x);
        by_portal.chain(within_layer)
    }

    pub fn get_loc(&self, index: usize) -> Option<PixelLoc> {
        self.size.get_loc(index)
    }

    pub fn len(&self) -> usize {
        self.size.len()
    }
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

    pub fn iter_adjacent(
        &self,
        loc: PixelLoc,
    ) -> impl Iterator<Item = PixelLoc> + '_ {
        (-1..=1)
            .cartesian_product(-1..=1)
            .filter(|&(di, dj)| (di != 0) || (dj != 0))
            .map(move |(di, dj)| PixelLoc {
                i: loc.i + di,
                j: loc.j + dj,
            })
            .filter(move |&loc| self.is_valid(loc))
    }

    pub fn get_loc(&self, index: usize) -> Option<PixelLoc> {
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

    #[test]
    fn test_line_to() -> Result<(), Error> {
        assert_eq!(
            PixelLoc { i: 0, j: 0 }.line_to(PixelLoc { i: 0, j: 0 }),
            vec![PixelLoc { i: 0, j: 0 }]
        );

        // Vertical line up
        assert_eq!(
            PixelLoc { i: 0, j: 0 }.line_to(PixelLoc { i: 0, j: 3 }),
            vec![
                PixelLoc { i: 0, j: 0 },
                PixelLoc { i: 0, j: 1 },
                PixelLoc { i: 0, j: 2 },
                PixelLoc { i: 0, j: 3 },
            ]
        );

        // Horizontal line right
        assert_eq!(
            PixelLoc { i: 0, j: 0 }.line_to(PixelLoc { i: 3, j: 0 }),
            vec![
                PixelLoc { i: 0, j: 0 },
                PixelLoc { i: 1, j: 0 },
                PixelLoc { i: 2, j: 0 },
                PixelLoc { i: 3, j: 0 },
            ]
        );

        // Diagonal line 1:1
        assert_eq!(
            PixelLoc { i: 0, j: 0 }.line_to(PixelLoc { i: 3, j: 3 }),
            vec![
                PixelLoc { i: 0, j: 0 },
                PixelLoc { i: 1, j: 0 },
                PixelLoc { i: 1, j: 1 },
                PixelLoc { i: 2, j: 1 },
                PixelLoc { i: 2, j: 2 },
                PixelLoc { i: 3, j: 2 },
                PixelLoc { i: 3, j: 3 },
            ]
        );

        // Slope < 1
        assert_eq!(
            PixelLoc { i: 0, j: 0 }.line_to(PixelLoc { i: 3, j: 2 }),
            vec![
                PixelLoc { i: 0, j: 0 },
                PixelLoc { i: 1, j: 0 },
                PixelLoc { i: 2, j: 0 },
                PixelLoc { i: 2, j: 1 },
                PixelLoc { i: 3, j: 1 },
                PixelLoc { i: 3, j: 2 },
            ]
        );

        // Slope > 1
        assert_eq!(
            PixelLoc { i: 0, j: 0 }.line_to(PixelLoc { i: 2, j: 3 }),
            vec![
                PixelLoc { i: 0, j: 0 },
                PixelLoc { i: 1, j: 0 },
                PixelLoc { i: 1, j: 1 },
                PixelLoc { i: 2, j: 1 },
                PixelLoc { i: 2, j: 2 },
                PixelLoc { i: 2, j: 3 },
            ]
        );

        // Off-origin
        assert_eq!(
            PixelLoc { i: 1, j: -1 }.line_to(PixelLoc { i: 3, j: 2 }),
            vec![
                PixelLoc { i: 1, j: -1 },
                PixelLoc { i: 2, j: -1 },
                PixelLoc { i: 2, j: 0 },
                PixelLoc { i: 3, j: 0 },
                PixelLoc { i: 3, j: 1 },
                PixelLoc { i: 3, j: 2 },
            ]
        );

        Ok(())
    }
}
