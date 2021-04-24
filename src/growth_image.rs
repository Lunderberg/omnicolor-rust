use std::collections::HashSet;
use std::path::PathBuf;

use itertools::Itertools;
use rand::distributions::Distribution;

use crate::color::RGB;
use crate::common::{PixelLoc, RectangularArray};
use crate::kd_tree::{KDTree, PerformanceStats, Point};
use crate::point_tracker::PointTracker;

impl Point for RGB {
    type Dtype = u8;
    const NUM_DIMENSIONS: u8 = 3;

    fn get_val(&self, dimension: u8) -> Self::Dtype {
        self.vals[dimension as usize]
    }

    fn dist2(&self, other: &Self) -> f64 {
        self.vals
            .iter()
            .zip(other.vals.iter())
            .map(|(a, b)| ((*a as f64) - (*b as f64)).powf(2.0))
            .sum()
    }
}

pub struct GrowthImage {
    pub(crate) size: RectangularArray,
    pub(crate) pixels: Vec<Option<RGB>>,
    pub(crate) stats: Vec<Option<PerformanceStats>>,
    pub(crate) num_filled_pixels: usize,

    pub(crate) stages: Vec<GrowthImageStage>,
    pub(crate) active_stage: Option<usize>,
    pub(crate) current_stage_iter: usize,

    pub(crate) point_tracker: PointTracker,
    pub(crate) epsilon: f64,

    pub(crate) is_done: bool,
}

pub struct GrowthImageStage {
    pub(crate) palette: KDTree<RGB>,
    pub(crate) max_iter: Option<usize>,
    pub(crate) grow_from_previous: bool,
    pub(crate) selected_seed_points: Vec<PixelLoc>,
    pub(crate) num_random_seed_points: u32,
}

impl GrowthImage {
    pub fn is_done(&self) -> bool {
        self.is_done
    }

    pub fn fill(&mut self) {
        let res = self.try_fill();
        self.is_done = res.is_none();
    }

    pub fn get_adjacent_color(&self, loc: PixelLoc) -> Option<RGB> {
        let (count, rsum, gsum, bsum) = (-1..=1)
            .cartesian_product(-1..=1)
            .filter(|(di, dj)| (*di != 0) || (*dj != 0))
            .map(|(di, dj)| PixelLoc {
                i: loc.i + di,
                j: loc.j + dj,
            })
            .flat_map(|loc| self.size.get_index(loc))
            .flat_map(|index| self.pixels[index])
            .fold(
                (0u32, 0u32, 0u32, 0u32),
                |(count, rsum, gsum, bsum), rgb| {
                    (
                        count + 1,
                        rsum + rgb.r() as u32,
                        gsum + rgb.g() as u32,
                        bsum + rgb.b() as u32,
                    )
                },
            );

        if count > 0 {
            Some(RGB {
                vals: [
                    (rsum / count) as u8,
                    (gsum / count) as u8,
                    (bsum / count) as u8,
                ],
            })
        } else {
            None
        }
    }

    fn current_stage_finished(&self) -> bool {
        let active_stage = &self.stages[self.active_stage.unwrap()];
        let reached_max_stage_iter = match active_stage.max_iter {
            Some(max_iter) => self.current_stage_iter >= max_iter,
            None => false,
        };
        let empty_palette = active_stage.palette.num_points() == 0;

        let empty_frontier = self.point_tracker.is_done();

        reached_max_stage_iter || empty_palette || empty_frontier
    }

    fn start_stage(&mut self, stage_index: usize) {
        // Advance stage number
        self.active_stage = Some(stage_index);
        self.current_stage_iter = 0;

        // Overkill at this point to remake the PointTracker, since we
        // could instead just clear the frontier when needed.  Once
        // forbidden points and portals are added, though, the
        // recalculating will be necessary.
        let mut point_tracker = PointTracker::new(self.size);
        let active_stage = &self.stages[stage_index];

        let filled_locs = self
            .pixels
            .iter()
            .enumerate()
            .filter(|(_i, p)| p.is_some())
            .flat_map(|(i, _p)| self.size.get_loc(i))
            .collect::<Vec<_>>();

        if active_stage.grow_from_previous {
            filled_locs
                .into_iter()
                .for_each(|loc| point_tracker.fill(loc));
        } else {
            filled_locs
                .into_iter()
                .for_each(|loc| point_tracker.mark_as_used(loc));
        };

        // Add in any selected seed points
        active_stage
            .selected_seed_points
            .iter()
            .for_each(|loc| point_tracker.add_to_frontier(*loc));

        // Randomly pick N seed points from those remaining.
        // Implementation assumes that N is relatively small, may be
        // inefficient for large N.
        let num_unfilled_pixels = self.pixels.len() - self.num_filled_pixels;
        let num_random = usize::min(
            active_stage.num_random_seed_points as usize,
            num_unfilled_pixels,
        );
        if num_random > 0 {
            let mut indices = HashSet::new();
            let mut rng = rand::thread_rng();
            let distribution =
                rand::distributions::Uniform::from(0..num_unfilled_pixels);
            while indices.len() < num_random {
                indices.insert(distribution.sample(&mut rng));
            }
            self.pixels
                .iter()
                .enumerate()
                .map(|(i, p)| (self.size.get_loc(i).unwrap(), p))
                .filter(|(_loc, p)| p.is_none())
                .map(|(loc, _p)| loc)
                .enumerate()
                .filter(|(i, _loc)| indices.contains(i))
                .for_each(|(_i, loc)| point_tracker.add_to_frontier(loc));
        }

        // Set the new point tracker as the one to use
        self.point_tracker = point_tracker;
    }

    fn try_fill(&mut self) -> Option<(PixelLoc, RGB)> {
        // Start of the first stage
        if self.active_stage.is_none() {
            self.start_stage(0);
        }

        // Advance to the next stage, if needed.
        while self.current_stage_finished() {
            let next_stage = self.active_stage.unwrap() + 1;
            if next_stage < self.stages.len() {
                self.start_stage(next_stage);
            } else {
                return None;
            }
        }

        let point_tracker_index = (self.point_tracker.frontier_size() as f32
            * rand::random::<f32>()) as usize;
        let next_loc =
            self.point_tracker.get_frontier_point(point_tracker_index);
        self.point_tracker.fill(next_loc);

        let next_index = self.size.get_index(next_loc)?;

        let target_color =
            self.get_adjacent_color(next_loc).unwrap_or_else(|| RGB {
                vals: [
                    rand::random::<u8>(),
                    rand::random::<u8>(),
                    rand::random::<u8>(),
                ],
            });

        let active_stage = &mut self.stages[self.active_stage.unwrap()];
        let res = active_stage
            .palette
            .pop_closest(&target_color, self.epsilon);
        self.stats[next_index] = Some(res.stats);

        let next_color = res.res?;
        self.pixels[next_index] = Some(next_color);

        self.current_stage_iter += 1;
        self.num_filled_pixels += 1;

        Some((next_loc, next_color))
    }

    pub fn write(&self, filename: &PathBuf) {
        let data = self
            .pixels
            .iter()
            .map(|p| match p {
                Some(rgb) => vec![rgb.r(), rgb.g(), rgb.b(), 255],
                None => vec![0, 0, 0, 0],
            })
            .flat_map(|p| p.into_iter())
            .collect::<Vec<u8>>();

        self.write_image(filename, &data);
    }

    pub fn write_stats(&self, filename: &PathBuf) {
        let max = self.stats.iter().filter_map(|s| *s).fold(
            PerformanceStats::default(),
            |a, b| PerformanceStats {
                nodes_checked: a.nodes_checked.max(b.nodes_checked),
                leaf_nodes_checked: a
                    .leaf_nodes_checked
                    .max(b.leaf_nodes_checked),
                points_checked: a.points_checked.max(b.points_checked),
            },
        );

        let data = self
            .stats
            .iter()
            .map(|s| match s {
                // Linear scaling from 0 to max.
                // Some(stats) => vec![
                //     (255 * stats.nodes_checked / max.nodes_checked) as u8,
                //     (255 * stats.leaf_nodes_checked / max.leaf_nodes_checked)
                //         as u8,
                //     (255 * stats.points_checked / max.points_checked) as u8,
                //     255,
                // ],
                Some(stats) => vec![
                    (255.0
                        * ((stats.nodes_checked as f32).ln()
                            / (max.nodes_checked as f32).ln()))
                        as u8,
                    (255.0
                        * ((stats.leaf_nodes_checked as f32).ln()
                            / (max.leaf_nodes_checked as f32).ln()))
                        as u8,
                    (255.0
                        * ((stats.points_checked as f32).ln()
                            / (max.points_checked as f32).ln()))
                        as u8,
                    255,
                ],
                None => vec![0, 0, 0, 0],
            })
            .flat_map(|p| p.into_iter())
            .collect::<Vec<u8>>();
        self.write_image(filename, &data)
    }

    fn write_image(&self, filename: &PathBuf, data: &[u8]) {
        let file = std::fs::File::create(filename).unwrap();
        let bufwriter = &mut std::io::BufWriter::new(file);

        let mut encoder =
            png::Encoder::new(bufwriter, self.size.width, self.size.height);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        writer.write_image_data(&data).unwrap();
    }
}
