use std::path::PathBuf;

use itertools::Itertools;

use crate::color::RGB;
use crate::common::{PixelLoc, RectangularArray};
use crate::errors::Error;
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

pub struct GrowthImageBuilder {
    size: RectangularArray,
    epsilon: f64,
    stages: Vec<GrowthImageStageBuilder>,
}

impl GrowthImageBuilder {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: RectangularArray { width, height },
            epsilon: 1.0,
            stages: Vec::new(),
        }
    }

    pub fn add_stage(mut self, stage: GrowthImageStageBuilder) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn epsilon(mut self, epsilon: f64) -> Self {
        self.epsilon = epsilon;
        self
    }

    pub fn palette(self, palette: Vec<RGB>) -> Self {
        self.add_stage(GrowthImageStageBuilder {
            palette,
            ..Default::default()
        })
    }

    pub fn build(self) -> Result<GrowthImage, Error> {
        if self.stages.len() == 0 {
            return Err(Error::NoStagesDefined);
        }

        let pixels = vec![None; self.size.len()];
        let stats = vec![None; self.size.len()];
        Ok(GrowthImage {
            size: self.size,
            pixels,
            stats,
            epsilon: self.epsilon,
            stages: self.stages.into_iter().map(|s| s.build()).collect(),
            active_stage: None,
            current_stage_iter: 0,
            point_tracker: PointTracker::new(self.size),
            done: false,
        })
    }
}

pub struct GrowthImageStageBuilder {
    pub palette: Vec<RGB>,
    pub max_iter: Option<usize>,
}

impl Default for GrowthImageStageBuilder {
    fn default() -> Self {
        Self {
            palette: Vec::new(),
            max_iter: None,
        }
    }
}

impl GrowthImageStageBuilder {
    fn build(self) -> GrowthImageStage {
        GrowthImageStage {
            palette: KDTree::new(self.palette),
            max_iter: self.max_iter,
        }
    }
}

struct GrowthImageStage {
    palette: KDTree<RGB>,
    max_iter: Option<usize>,
}

pub struct GrowthImage {
    size: RectangularArray,

    pixels: Vec<Option<RGB>>,
    stats: Vec<Option<PerformanceStats>>,

    stages: Vec<GrowthImageStage>,
    active_stage: Option<usize>,
    current_stage_iter: usize,

    point_tracker: PointTracker,

    epsilon: f64,

    pub done: bool,
}

impl GrowthImage {
    pub fn fill(&mut self) {
        let res = self.try_fill();
        self.done = res.is_none();
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

    fn start_stage(&mut self, stage_index: usize) {
        // Advance stage number
        self.active_stage = Some(stage_index);
        self.current_stage_iter = 0;

        // No frontier on first stage, everything empty
        if stage_index == 0 && self.point_tracker.frontier_size() == 0 {
            let first_frontier = self.size.get_random_loc();
            self.point_tracker.add_to_frontier(first_frontier);
        }
    }

    fn try_fill(&mut self) -> Option<(PixelLoc, RGB)> {
        // Start of the first stage
        if self.active_stage.is_none() {
            self.start_stage(0);
        }

        // Check if stage is finished
        {
            let active_stage = &self.stages[self.active_stage.unwrap()];
            let reached_max_stage_iter = match active_stage.max_iter {
                Some(max_iter) => self.current_stage_iter >= max_iter,
                None => false,
            };
            let empty_palette = active_stage.palette.num_points() == 0;

            if reached_max_stage_iter || empty_palette {
                let next_stage = self.active_stage.unwrap() + 1;
                if next_stage < self.stages.len() {
                    self.start_stage(next_stage);
                } else {
                    return None;
                }
            }
        }

        // No frontier, everything full
        if self.point_tracker.done {
            return None;
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
