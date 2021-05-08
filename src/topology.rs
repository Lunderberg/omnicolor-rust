use std::collections::HashMap;
use std::ops::Range;

use itertools::Itertools;

#[allow(unused_imports)]
use crate::errors::Error;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct PixelLoc {
    pub layer: u8,
    pub i: i32,
    pub j: i32,
}

impl PixelLoc {
    // Line between two points.  Uses Bresenham's, modified to have no
    // diagonal openings.  Assumes the two points are on the same layer.
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
                output.push(PixelLoc {
                    layer: self.layer,
                    i,
                    j,
                });
            }

            prev_j = Some(j1);
        }

        output
    }

    fn vertical_line_to(&self, other_j: i32) -> Vec<PixelLoc> {
        let jmin = self.j.min(other_j);
        let jmax = self.j.max(other_j);
        (jmin..=jmax)
            .map(|j| PixelLoc {
                layer: self.layer,
                i: self.i,
                j,
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct Topology {
    pub layers: Vec<RectangularArray>,
    pub portals: HashMap<PixelLoc, PixelLoc>,
}

// Currently, most of these just delegate to RectangularArray, but
// they'll be more differentiated once there are multiple layers to
// the image.
impl Topology {
    pub fn is_valid(&self, loc: PixelLoc) -> bool {
        self.layers
            .get(loc.layer as usize)
            .map(|layer| layer.is_valid(loc))
            .unwrap_or(false)
    }

    // Return the index associated with a pixel location, or None if
    // the location is invalid (e.g. no such layer, or out of bounds
    // for that layer).
    pub fn get_index(&self, loc: PixelLoc) -> Option<usize> {
        // Allow for a flat array of pixels to store all layers
        self.layers
            .get(loc.layer as usize)
            .map(|layer| {
                layer.get_index(loc).map(|in_layer_index| {
                    let offset = self.layers[0..(loc.layer as usize)]
                        .iter()
                        .map(|prev_layer| prev_layer.len())
                        .sum::<usize>();
                    in_layer_index + offset
                })
            })
            .flatten()
    }

    pub fn iter_adjacent(
        &self,
        loc: PixelLoc,
    ) -> impl Iterator<Item = PixelLoc> + '_ {
        let within_layer = self
            .layers
            .get(loc.layer as usize)
            .map(|layer| layer.iter_adjacent(loc))
            .into_iter()
            .flatten();
        let by_portal = self.portals.get(&loc).into_iter().map(|x| *x);
        by_portal.chain(within_layer)
    }

    pub fn get_layer_bounds(&self, layer: u8) -> Option<Range<usize>> {
        let layer = layer as usize;
        if layer < self.layers.len() {
            let offset = self.layers[0..layer]
                .iter()
                .map(|prev_layer| prev_layer.len())
                .sum::<usize>();
            let len = self.layers[layer].len();
            Some(offset..(offset + len))
        } else {
            None
        }
    }

    pub fn get_loc(&self, index: usize) -> Option<PixelLoc> {
        self.layers
            .iter()
            .enumerate()
            .scan(0, |cumsum, (layer_i, layer)| {
                let min_index = *cumsum;
                *cumsum = min_index + layer.len();
                let max_index = *cumsum;
                Some((min_index, max_index, layer, layer_i))
            })
            .filter(|&(min_index, max_index, _layer, _layer_i)| {
                index >= min_index && index < max_index
            })
            .next()
            .map(|(min_index, _max_index, layer, layer_i)| {
                layer.get_loc(layer_i as u8, index - min_index)
            })
            .flatten()
    }

    pub fn len(&self) -> usize {
        self.layers.iter().map(|layer| layer.len()).sum()
    }
}

#[derive(Debug, Clone, Copy)]
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
                layer: loc.layer,
                i: loc.i + di,
                j: loc.j + dj,
            })
            .filter(move |&loc| self.is_valid(loc))
    }

    pub fn get_loc(&self, layer: u8, index: usize) -> Option<PixelLoc> {
        if index < self.len() {
            Some(PixelLoc {
                layer,
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
        let layer = 0u8;
        assert!(size.is_valid(PixelLoc { layer, i: 2, j: 3 }));
        assert!(size.is_valid(PixelLoc { layer, i: 4, j: 9 }));
        assert!(size.is_valid(PixelLoc { layer, i: 0, j: 0 }));

        assert!(!size.is_valid(PixelLoc { layer, i: 5, j: 3 }));
        assert!(!size.is_valid(PixelLoc { layer, i: 2, j: 10 }));
        assert!(!size.is_valid(PixelLoc { layer, i: 2, j: 15 }));
        assert!(!size.is_valid(PixelLoc { layer, i: -1, j: 3 }));
        assert!(!size.is_valid(PixelLoc { layer, i: 5, j: -1 }));
        assert!(!size.is_valid(PixelLoc { layer, i: 2, j: -1 }));
        assert!(!size.is_valid(PixelLoc {
            layer,
            i: -1,
            j: -1
        }));

        Ok(())
    }

    #[test]
    fn test_index_lookup() -> Result<(), Error> {
        let size = RectangularArray {
            width: 5,
            height: 10,
        };

        let layer = 0u8;

        assert_eq!(size.get_index(PixelLoc { layer, i: 0, j: 0 }), Some(0));
        assert_eq!(size.get_index(PixelLoc { layer, i: 1, j: 0 }), Some(1));
        assert_eq!(size.get_index(PixelLoc { layer, i: 0, j: 1 }), Some(5));
        assert_eq!(size.get_index(PixelLoc { layer, i: 1, j: 1 }), Some(6));
        assert_eq!(size.get_index(PixelLoc { layer, i: 4, j: 9 }), Some(49));

        assert_eq!(size.get_index(PixelLoc { layer, i: -1, j: 1 }), None);
        assert_eq!(size.get_index(PixelLoc { layer, i: 4, j: 10 }), None);

        assert_eq!(
            size.get_loc(layer, 0),
            Some(PixelLoc { layer, i: 0, j: 0 })
        );
        assert_eq!(
            size.get_loc(layer, 11),
            Some(PixelLoc { layer, i: 1, j: 2 })
        );
        assert_eq!(
            size.get_loc(layer, 1),
            Some(PixelLoc { layer, i: 1, j: 0 })
        );
        assert_eq!(size.get_loc(layer, 50), None);
        assert_eq!(size.get_loc(layer, 500000), None);

        Ok(())
    }

    #[test]
    fn test_line_to() -> Result<(), Error> {
        let layer = 0u8;

        assert_eq!(
            PixelLoc { layer, i: 0, j: 0 }.line_to(PixelLoc {
                layer,
                i: 0,
                j: 0
            }),
            vec![PixelLoc { layer, i: 0, j: 0 }]
        );

        // Vertical line up
        assert_eq!(
            PixelLoc { layer, i: 0, j: 0 }.line_to(PixelLoc {
                layer,
                i: 0,
                j: 3
            }),
            vec![
                PixelLoc { layer, i: 0, j: 0 },
                PixelLoc { layer, i: 0, j: 1 },
                PixelLoc { layer, i: 0, j: 2 },
                PixelLoc { layer, i: 0, j: 3 },
            ]
        );

        // Horizontal line right
        assert_eq!(
            PixelLoc { layer, i: 0, j: 0 }.line_to(PixelLoc {
                layer,
                i: 3,
                j: 0
            }),
            vec![
                PixelLoc { layer, i: 0, j: 0 },
                PixelLoc { layer, i: 1, j: 0 },
                PixelLoc { layer, i: 2, j: 0 },
                PixelLoc { layer, i: 3, j: 0 },
            ]
        );

        // Diagonal line 1:1
        assert_eq!(
            PixelLoc { layer, i: 0, j: 0 }.line_to(PixelLoc {
                layer,
                i: 3,
                j: 3
            }),
            vec![
                PixelLoc { layer, i: 0, j: 0 },
                PixelLoc { layer, i: 1, j: 0 },
                PixelLoc { layer, i: 1, j: 1 },
                PixelLoc { layer, i: 2, j: 1 },
                PixelLoc { layer, i: 2, j: 2 },
                PixelLoc { layer, i: 3, j: 2 },
                PixelLoc { layer, i: 3, j: 3 },
            ]
        );

        // Slope < 1
        assert_eq!(
            PixelLoc { layer, i: 0, j: 0 }.line_to(PixelLoc {
                layer,
                i: 3,
                j: 2
            }),
            vec![
                PixelLoc { layer, i: 0, j: 0 },
                PixelLoc { layer, i: 1, j: 0 },
                PixelLoc { layer, i: 2, j: 0 },
                PixelLoc { layer, i: 2, j: 1 },
                PixelLoc { layer, i: 3, j: 1 },
                PixelLoc { layer, i: 3, j: 2 },
            ]
        );

        // Slope > 1
        assert_eq!(
            PixelLoc { layer, i: 0, j: 0 }.line_to(PixelLoc {
                layer,
                i: 2,
                j: 3
            }),
            vec![
                PixelLoc { layer, i: 0, j: 0 },
                PixelLoc { layer, i: 1, j: 0 },
                PixelLoc { layer, i: 1, j: 1 },
                PixelLoc { layer, i: 2, j: 1 },
                PixelLoc { layer, i: 2, j: 2 },
                PixelLoc { layer, i: 2, j: 3 },
            ]
        );

        // Off-origin
        assert_eq!(
            PixelLoc { layer, i: 1, j: -1 }.line_to(PixelLoc {
                layer,
                i: 3,
                j: 2
            }),
            vec![
                PixelLoc { layer, i: 1, j: -1 },
                PixelLoc { layer, i: 2, j: -1 },
                PixelLoc { layer, i: 2, j: 0 },
                PixelLoc { layer, i: 3, j: 0 },
                PixelLoc { layer, i: 3, j: 1 },
                PixelLoc { layer, i: 3, j: 2 },
            ]
        );

        Ok(())
    }

    #[test]
    fn test_topology_index_lookup() -> Result<(), Error> {
        let topology = Topology {
            layers: vec![
                RectangularArray {
                    width: 10,
                    height: 10,
                },
                RectangularArray {
                    width: 5,
                    height: 5,
                },
            ],
            portals: HashMap::new(),
        };

        assert_eq!(
            topology.get_loc(0),
            Some(PixelLoc {
                layer: 0,
                i: 0,
                j: 0
            })
        );

        assert_eq!(
            topology.get_loc(100),
            Some(PixelLoc {
                layer: 1,
                i: 0,
                j: 0
            })
        );

        Ok(())
    }
}
