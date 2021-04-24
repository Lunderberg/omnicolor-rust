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
    epsilon: f32,
    palette: Option<Vec<RGB>>,
}

impl GrowthImageBuilder {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: RectangularArray { width, height },
            epsilon: 1.0,
            palette: None,
        }
    }

    pub fn epsilon(mut self, epsilon: f32) -> Self {
        self.epsilon = epsilon;
        self
    }

    pub fn palette(mut self, palette: Vec<RGB>) -> Self {
        self.palette = Some(palette);
        self
    }

    pub fn build(self) -> Result<GrowthImage, Error> {
        let palette = self.palette.ok_or(Error::NoPaletteDefined)?;
        let pixels = vec![None; self.size.len()];
        let stats = vec![None; self.size.len()];
        let palette = KDTree::new(palette, self.epsilon);
        Ok(GrowthImage {
            size: self.size,
            pixels,
            stats,
            palette,
            point_tracker: PointTracker::new(self.size),
            done: false,
        })
    }
}

pub struct GrowthImage {
    size: RectangularArray,

    pixels: Vec<Option<RGB>>,
    stats: Vec<Option<PerformanceStats>>,
    palette: KDTree<RGB>,
    point_tracker: PointTracker,

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

    fn try_fill(&mut self) -> Option<(PixelLoc, RGB)> {
        // No frontier, everything full
        if self.point_tracker.done {
            return None;
        }

        // No frontier, everything empty
        if self.point_tracker.frontier_size() == 0 {
            let first_frontier = self.size.get_random_loc();
            self.point_tracker.add_to_frontier(first_frontier);
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

        let res = self.palette.pop_closest(&target_color);
        self.stats[next_index] = Some(res.stats);

        let next_color = res.res?;
        self.pixels[next_index] = Some(next_color);

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
