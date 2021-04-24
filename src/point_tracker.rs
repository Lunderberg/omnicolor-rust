use std::collections::HashMap;

use itertools::Itertools;

use crate::common::PixelLoc;

pub struct PointTracker {
    frontier: Vec<PixelLoc>,
    frontier_map: HashMap<PixelLoc, usize>,
    used: Vec<bool>,
    width: u32,
    height: u32,
    pub done: bool,
}

impl PointTracker {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            frontier: Vec::new(),
            frontier_map: HashMap::new(),
            used: vec![false; (width * height) as usize],
            done: false,
        }
    }

    pub fn add_to_frontier(&mut self, loc: PixelLoc) {
        let index = (loc.j * (self.width as i32) + loc.i) as usize;
        if !self.used[index] {
            self.frontier_map.insert(loc, self.frontier.len());
            self.frontier.push(loc);
            self.used[index] = true;
        }
    }

    pub fn frontier_size(&self) -> usize {
        self.frontier.len()
    }

    pub fn get_frontier_point(&self, index: usize) -> PixelLoc {
        self.frontier[index]
    }

    pub fn fill(&mut self, loc: PixelLoc) {
        let width = self.width as i32;
        let height = self.height as i32;
        (-1..=1)
            .cartesian_product(-1..=1)
            .map(|(di, dj)| PixelLoc {
                i: loc.i + di,
                j: loc.j + dj,
            })
            .filter(|adjacent| {
                (adjacent.i >= 0)
                    && (adjacent.j >= 0)
                    && (adjacent.i < width)
                    && (adjacent.j < height)
            })
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
