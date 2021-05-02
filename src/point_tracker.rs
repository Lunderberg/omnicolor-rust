use std::collections::{HashMap, HashSet};

use rand::distributions::Distribution;
use rand::Rng;

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
        let index = self.topology.get_index(loc);
        if let Some(index) = index {
            PointTracker::_add_to_frontier(
                &mut self.frontier,
                &mut self.frontier_map,
                &mut self.used,
                index,
                loc,
            );
        }
    }

    pub fn add_random_to_frontier(
        &mut self,
        num_random: usize,
        rng: &mut impl Rng,
    ) {
        let num_unused = self.used.iter().filter(|&x| !x).count();

        let num_random = usize::min(num_unused, num_random);

        if num_random == 0 {
            return;
        }

        let mut indices = HashSet::new();
        let distribution = rand::distributions::Uniform::from(0..num_unused);
        while indices.len() < num_random {
            indices.insert(distribution.sample(rng));
        }
        self.used
            .iter()
            .enumerate()
            .filter(|(_i, &b)| !b)
            .map(|(i, _b)| i)
            .enumerate()
            .filter(|(i_unused, _i_arr)| indices.contains(i_unused))
            .map(|(_i_unused, i_arr)| {
                (i_arr, self.topology.get_loc(i_arr).unwrap())
            })
            .collect::<Vec<_>>()
            .iter()
            .for_each(|&(i_arr, loc)| {
                PointTracker::_add_to_frontier(
                    &mut self.frontier,
                    &mut self.frontier_map,
                    &mut self.used,
                    i_arr,
                    loc,
                )
            });
    }

    fn _add_to_frontier(
        frontier: &mut Vec<PixelLoc>,
        frontier_map: &mut HashMap<PixelLoc, usize>,
        used: &mut Vec<bool>,
        index: usize,
        loc: PixelLoc,
    ) {
        if !used[index] {
            frontier_map.insert(loc, frontier.len());
            frontier.push(loc);
            used[index] = true;
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
            let index = topology.get_index(adjacent);
            if let Some(index) = index {
                PointTracker::_add_to_frontier(
                    &mut frontier,
                    &mut frontier_map,
                    &mut used,
                    index,
                    adjacent,
                );
            }
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
