use std::collections::HashMap;

use itertools::Itertools;

use crate::common::{PixelLoc, RectangularArray};

pub struct PointTracker {
    frontier: Vec<PixelLoc>,
    frontier_map: HashMap<PixelLoc, usize>,
    used: Vec<bool>,
    size: RectangularArray,
    pub done: bool,
}

impl PointTracker {
    pub fn new(size: RectangularArray) -> Self {
        Self {
            size,
            frontier: Vec::new(),
            frontier_map: HashMap::new(),
            used: vec![false; size.len()],
            done: false,
        }
    }

    pub fn add_to_frontier(&mut self, loc: PixelLoc) {
        let index = self.size.get_index(loc);
        if let Some(index) = index {
            if !self.used[index] {
                self.frontier_map.insert(loc, self.frontier.len());
                self.frontier.push(loc);
                self.used[index] = true;
            }
        }
    }

    pub fn frontier_size(&self) -> usize {
        self.frontier.len()
    }

    pub fn get_frontier_point(&self, index: usize) -> PixelLoc {
        self.frontier[index]
    }

    pub fn fill(&mut self, loc: PixelLoc) {
        let size = self.size;
        (-1..=1)
            .cartesian_product(-1..=1)
            .map(|(di, dj)| PixelLoc {
                i: loc.i + di,
                j: loc.j + dj,
            })
            .filter(|adjacent| size.is_valid(*adjacent))
            .for_each(|adjacent| self.add_to_frontier(adjacent));

        self.remove_from_frontier(loc);

        if self.frontier_size() == 0 {
            self.done = true;
        }
    }

    fn remove_from_frontier(&mut self, loc: PixelLoc) {
        let index = self.frontier_map.get(&loc).map(|i| *i);
        if let Some(index) = index {
            let last_point = *self.frontier.last().unwrap();
            self.frontier_map.insert(last_point, index);
            self.frontier.swap_remove(index);
            self.frontier_map.remove(&loc);
        }
    }
}
