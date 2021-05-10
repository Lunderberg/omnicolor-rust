use std::collections::HashMap;
use std::path::PathBuf;

use indicatif::ProgressBar;
use rand::Rng;

use crate::color::RGB;
use crate::kd_tree::{KDTree, PerformanceStats, Point};
use crate::point_tracker::PointTracker;
use crate::topology::{PixelLoc, Topology};

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
            .map(|(&a, &b)| ((a as f64) - (b as f64)).powf(2.0))
            .sum()
    }
}

pub struct GrowthImage {
    pub(crate) topology: Topology,
    pub(crate) pixels: Vec<Option<RGB>>,
    pub(crate) stats: Vec<Option<PerformanceStats>>,
    pub(crate) num_filled_pixels: usize,

    pub(crate) stages: Vec<GrowthImageStage>,
    pub(crate) active_stage: Option<usize>,
    pub(crate) current_stage_iter: usize,

    pub(crate) point_tracker: PointTracker,
    pub(crate) epsilon: f64,
    pub(crate) rng: rand_chacha::ChaCha8Rng,

    pub(crate) is_done: bool,
    pub(crate) progress_bar: Option<ProgressBar>,
    pub(crate) animation_outputs: Vec<GrowthImageAnimation>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SaveImageType {
    Generated,
    Statistics,
    ColorPalette,
}

struct SaveImageData {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

#[derive(Clone)]
pub enum RestrictedRegion {
    Allowed(Vec<PixelLoc>),
    Forbidden(Vec<PixelLoc>),
}

pub struct GrowthImageStage {
    pub(crate) palette: KDTree<RGB>,
    pub(crate) max_iter: Option<usize>,
    pub(crate) grow_from_previous: bool,
    pub(crate) selected_seed_points: Vec<PixelLoc>,
    pub(crate) num_random_seed_points: u32,
    pub(crate) restricted_region: RestrictedRegion,
    pub(crate) portals: HashMap<PixelLoc, PixelLoc>,
}

pub struct GrowthImageAnimation {
    pub(crate) proc: std::process::Child,
    pub(crate) iter_per_frame: usize,
    pub(crate) image_type: SaveImageType,
    pub(crate) layer: u8,
}

impl GrowthImage {
    pub fn is_done(&self) -> bool {
        self.is_done
    }

    pub fn fill_until_done(&mut self) {
        while !self.is_done {
            self.fill();
        }
    }

    pub fn fill(&mut self) {
        let res = self.try_fill();
        self.is_done = res.is_none();

        if let Some(bar) = &self.progress_bar {
            bar.inc(1);
            if self.is_done {
                bar.finish();
            }
        }

        self._write_to_animations();
    }

    pub fn get_adjacent_color(&self, loc: PixelLoc) -> Option<RGB> {
        let (count, rsum, gsum, bsum) = self
            .topology
            .iter_adjacent(loc)
            .flat_map(|loc| self.topology.get_index(loc))
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
        let active_stage = &self.stages[stage_index];

        // Update the geometry with new portals.  Long-term, should
        // forbidden points go here as well?  Conceptually, they fit
        // really well with the geometry tracking class, but the
        // implementation is much cleaner with them being part of the
        // PointTracker's "used" array.
        self.topology.portals = active_stage.portals.clone();

        // Remake the PointTracker, so that we can clear any forbidden
        // points from the previous stage, as well as removing any
        // newly forbidden points from the frontier.
        let mut point_tracker = PointTracker::new(self.topology.clone());

        match &active_stage.restricted_region {
            RestrictedRegion::Allowed(points) => {
                point_tracker.mark_all_as_used();
                points
                    .iter()
                    .for_each(|&loc| point_tracker.mark_as_unused(loc));
            }
            RestrictedRegion::Forbidden(points) => {
                points
                    .iter()
                    .for_each(|&loc| point_tracker.mark_as_used(loc));
            }
        }

        // Any additionally specified pixels are forbidden

        // All filled pixels are either forbidden, or forbidden with a
        // frontier.
        let filled_locs = self
            .pixels
            .iter()
            .enumerate()
            .filter(|(_i, p)| p.is_some())
            .flat_map(|(i, _p)| self.topology.get_loc(i));

        if active_stage.grow_from_previous {
            filled_locs.for_each(|loc| point_tracker.fill(loc));
        } else {
            filled_locs.for_each(|loc| point_tracker.mark_as_used(loc));
        };

        // Add in any selected seed points
        active_stage
            .selected_seed_points
            .iter()
            .for_each(|&loc| point_tracker.add_to_frontier(loc));

        // Randomly pick N seed points from those remaining.
        // Implementation assumes that N is relatively small, may be
        // inefficient for large N.
        point_tracker.add_random_to_frontier(
            active_stage.num_random_seed_points as usize,
            &mut self.rng,
        );

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
            * self.rng.gen::<f32>()) as usize;
        let next_loc =
            self.point_tracker.get_frontier_point(point_tracker_index);
        self.point_tracker.fill(next_loc);

        let next_index = self.topology.get_index(next_loc)?;

        let target_color =
            self.get_adjacent_color(next_loc).unwrap_or_else(|| RGB {
                vals: [
                    self.rng.gen::<u8>(),
                    self.rng.gen::<u8>(),
                    self.rng.gen::<u8>(),
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

    pub fn write(&self, filename: PathBuf) {
        self.write_image(filename, SaveImageType::Generated, 0);
    }

    pub fn write_image(
        &self,
        filename: PathBuf,
        image_type: SaveImageType,
        layer: u8,
    ) {
        self._write_image_data(filename, &self._image_data(image_type, layer));
    }

    fn _write_to_animations(&mut self) {
        // Steal the stdin from the GrowthImageAnimations
        let mut stdin_list: Vec<_> = self
            .animation_outputs
            .iter_mut()
            .map(|anim| anim.proc.stdin.take().unwrap())
            .collect();

        // Write to it, which requires immutable borrow of other parts
        // of self.
        self.animation_outputs
            .iter()
            .zip(stdin_list.iter_mut())
            .filter(|(anim, _stdin)| {
                (self.num_filled_pixels - 1) % anim.iter_per_frame == 0
            })
            .for_each(|(anim, stdin)| {
                let data = self._image_data(anim.image_type, anim.layer);
                self._write_image_data_to_writer(stdin, &data);
            });

        // Put the stdin back into the GrowthImageAnimations
        self.animation_outputs
            .iter_mut()
            .zip(stdin_list.into_iter())
            .for_each(|(anim, stdin)| {
                anim.proc.stdin.replace(stdin);
            });
    }

    fn _image_data(
        &self,
        image_type: SaveImageType,
        layer: u8,
    ) -> SaveImageData {
        match image_type {
            SaveImageType::Generated => self._generated_image_data(layer),
            SaveImageType::Statistics => self._statistics_image_data(layer),
            SaveImageType::ColorPalette => self._color_palette_image_data(),
        }
    }

    fn _generated_image_data(&self, layer: u8) -> SaveImageData {
        let index_range = self.topology.get_layer_bounds(layer).unwrap();
        let size = self.topology.layers[layer as usize];
        let data = self.pixels[index_range]
            .iter()
            .map(|p| match p {
                Some(rgb) => vec![rgb.r(), rgb.g(), rgb.b(), 255],
                None => vec![0, 0, 0, 0],
            })
            .flat_map(|p| p.into_iter())
            .collect();
        SaveImageData {
            data,
            width: size.width,
            height: size.height,
        }
    }

    fn _statistics_image_data(&self, layer: u8) -> SaveImageData {
        let index_range = self.topology.get_layer_bounds(layer).unwrap();
        let size = self.topology.layers[layer as usize];
        let max = self.stats[index_range.clone()]
            .iter()
            .filter_map(|s| *s)
            .fold(PerformanceStats::default(), |a, b| PerformanceStats {
                nodes_checked: a.nodes_checked.max(b.nodes_checked),
                leaf_nodes_checked: a
                    .leaf_nodes_checked
                    .max(b.leaf_nodes_checked),
                points_checked: a.points_checked.max(b.points_checked),
            });

        let data = self.stats[index_range]
            .iter()
            .map(|s| match s {
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
            .collect();

        SaveImageData {
            data,
            width: size.width,
            height: size.height,
        }
    }

    fn _color_palette_image_data(&self) -> SaveImageData {
        let mut data = self.stages[self.active_stage.unwrap_or(0)]
            .palette
            .iter_points()
            .map(|p| match p {
                Some(rgb) => vec![rgb.r(), rgb.g(), rgb.b(), 255],
                None => vec![0, 0, 0, 0],
            })
            .flat_map(|p| p.into_iter())
            .collect::<Vec<u8>>();

        // TODO: Better method here.  Currently, the smallest size
        // with enough points that roughly matches the aspect
        // ratio of layer 0.
        let aspect_ratio = (self.topology.layers[0].width as f64)
            / (self.topology.layers[0].height as f64);

        let area = self.topology.len() as f64;
        let height = (area / aspect_ratio).sqrt();
        let width = (height * aspect_ratio).ceil() as u32;
        let height = height.ceil() as u32;

        // Pad data array out with 0 as needed.
        data.resize((4 * width * height) as usize, 0);

        SaveImageData {
            data,
            width,
            height,
        }
    }

    fn _write_image_data(&self, filename: PathBuf, data: &SaveImageData) {
        let file = std::fs::File::create(filename).unwrap();
        let bufwriter = &mut std::io::BufWriter::new(file);

        self._write_image_data_to_writer(bufwriter, data);
    }

    fn _write_image_data_to_writer(
        &self,
        writer: &mut impl std::io::Write,
        data: &SaveImageData,
    ) {
        let mut encoder = png::Encoder::new(writer, data.width, data.height);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        writer.write_image_data(&data.data).unwrap();
    }
}

impl Drop for GrowthImage {
    fn drop(&mut self) {
        self.animation_outputs.iter_mut().for_each(|anim| {
            anim.proc.wait().unwrap();
        });
    }
}
