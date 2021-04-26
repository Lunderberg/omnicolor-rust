use std::collections::HashMap;

use crate::topology::{PixelLoc, Topology};

pub struct PointTracker {
    frontier: Vec<PixelLoc>,
    frontier_map: HashMap<PixelLoc, usize>,
    used: Vec<bool>,
    topology: Topology,
}

impl PointTracker {
    pub fn new(topology: Topology) -> Self {
        Self {
            used: vec![false; topology.len()],
            topology,
            frontier: Vec::new(),
            frontier_map: HashMap::new(),
        }
    }

    pub fn add_to_frontier(&mut self, loc: PixelLoc) {
        PointTracker::_add_to_frontier(
            &mut self.frontier,
            &mut self.frontier_map,
            &mut self.used,
            &self.topology,
            loc,
        );
    }

    fn _add_to_frontier(
        frontier: &mut Vec<PixelLoc>,
        frontier_map: &mut HashMap<PixelLoc, usize>,
        used: &mut Vec<bool>,
        topology: &Topology,
        loc: PixelLoc,
    ) {
        let index = topology.get_index(loc);
        if let Some(index) = index {
            if !used[index] {
                frontier_map.insert(loc, frontier.len());
                frontier.push(loc);
                used[index] = true;
            }
        }
    }

    pub fn mark_as_used(&mut self, loc: PixelLoc) {
        let index = self.topology.get_index(loc);
        if let Some(index) = index {
            self.used[index] = true;
        }
    }

    pub fn is_done(&self) -> bool {
        return self.frontier.len() == 0;
    }

    pub fn frontier_size(&self) -> usize {
        self.frontier.len()
    }

    pub fn get_frontier_point(&self, index: usize) -> PixelLoc {
        self.frontier[index]
    }

    pub fn fill(&mut self, loc: PixelLoc) {
        let topology = &self.topology;
        let mut frontier = &mut self.frontier;
        let mut frontier_map = &mut self.frontier_map;
        let mut used = &mut self.used;

        topology.iter_adjacent(loc).for_each(|adjacent| {
            PointTracker::_add_to_frontier(
                &mut frontier,
                &mut frontier_map,
                &mut used,
                &topology,
                adjacent,
            )
        });

        self.remove_from_frontier(loc);
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
