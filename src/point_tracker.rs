use std::collections::HashMap;

use itertools::Itertools;

pub struct PointTracker {
    frontier: Vec<(u32, u32)>,
    frontier_map: HashMap<(u32, u32), usize>,
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

    pub fn add_to_frontier(&mut self, i: u32, j: u32) {
        let index = (j * self.width + i) as usize;
        if !self.used[index] {
            self.frontier_map.insert((i, j), self.frontier.len());
            self.frontier.push((i, j));
            self.used[index] = true;
        }
    }

    pub fn frontier_size(&self) -> usize {
        self.frontier.len()
    }

    pub fn get_frontier_point(&self, i: usize) -> (u32, u32) {
        self.frontier[i]
    }

    pub fn fill(&mut self, i: u32, j: u32) {
        let width = self.width as i32;
        let height = self.height as i32;
        (-1..=1)
            .cartesian_product(-1..=1)
            .map(|(di, dj)| ((i as i32) + di, (j as i32) + dj))
            .filter(|(i, j)| {
                (*i >= 0) && (*j >= 0) && (*i < width) && (*j < height)
            })
            .for_each(|(i, j)| self.add_to_frontier(i as u32, j as u32));

        self.remove_from_frontier(i, j);

        if self.frontier_size() == 0 {
            self.done = true;
        }
    }

    fn remove_from_frontier(&mut self, i: u32, j: u32) {
        let index = self.frontier_map.get(&(i, j)).map(|i| *i);
        if let Some(index) = index {
            let last_point = *self.frontier.last().unwrap();
            self.frontier_map.insert(last_point, index);
            self.frontier.swap_remove(index);
            self.frontier_map.remove(&(i, j));
        }
    }
}
