use std::collections::HashMap;
use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};
use rand::{Rng, SeedableRng};

use crate::errors::Error;
use crate::growth_image::{
    GrowthImage, GrowthImageAnimation, GrowthImageStage, SaveImageType,
};
use crate::kd_tree::KDTree;
use crate::palettes::{Palette, UniformPalette};
use crate::point_tracker::PointTracker;
use crate::topology::{PixelLoc, RectangularArray, Topology};

pub struct GrowthImageBuilder {
    topology: Topology,
    epsilon: f64,
    stages: Vec<GrowthImageStageBuilder>,
    seed: Option<u64>,
    show_progress_bar: bool,

    animation_outputs: Vec<GrowthImageAnimationBuilder>,
}

impl GrowthImageBuilder {
    pub fn new() -> Self {
        let topology = Topology {
            layers: Vec::new(),
            portals: HashMap::new(),
        };

        Self {
            topology,
            epsilon: 1.0,
            stages: Vec::new(),
            seed: None,
            show_progress_bar: false,
            animation_outputs: Vec::new(),
        }
    }

    pub fn show_progress_bar(&mut self) -> &mut Self {
        self.show_progress_bar = true;
        self
    }

    pub fn add_output_animation(
        &mut self,
        filename: PathBuf,
    ) -> &mut GrowthImageAnimationBuilder {
        let new_stage = GrowthImageAnimationBuilder::new(filename);
        self.animation_outputs.push(new_stage);
        self.animation_outputs.last_mut().unwrap()
    }

    pub fn add_layer(&mut self, width: u32, height: u32) -> &mut Self {
        self.topology
            .layers
            .push(RectangularArray { width, height });
        self
    }

    pub fn new_stage(&mut self) -> &mut GrowthImageStageBuilder {
        let new_stage = GrowthImageStageBuilder::new(self.stages.len());
        self.stages.push(new_stage);
        self.stages.last_mut().unwrap()
    }

    pub fn epsilon(&mut self, epsilon: f64) -> &mut Self {
        self.epsilon = epsilon;
        self
    }

    pub fn palette<T>(&mut self, palette: T) -> &mut Self
    where
        T: Palette + Sized + 'static,
    {
        self.new_stage().palette(palette);
        self
    }

    pub fn seed(&mut self, seed: u64) -> &mut Self {
        self.seed = Some(seed);
        self
    }

    pub fn build(&self) -> Result<GrowthImage, Error> {
        if self.stages.len() == 0 {
            return Err(Error::NoStagesDefined);
        }
        if self.topology.len() == 0 {
            return Err(Error::NoLayersDefined);
        }

        let mut rng = match self.seed {
            Some(seed) => rand_chacha::ChaCha8Rng::seed_from_u64(seed),
            None => rand_chacha::ChaCha8Rng::from_entropy(),
        };

        let pixels = vec![None; self.topology.len()];
        let stats = vec![None; self.topology.len()];
        let stages = self
            .stages
            .iter()
            .map(|s| s.build(&self.topology, &mut rng))
            .collect();

        let progress_bar = if self.show_progress_bar {
            let bar = ProgressBar::new(self.topology.len() as u64);
            bar.set_style(ProgressStyle::default_bar().template(
                "[{pos}/{len}] {wide_bar} [{elapsed_precise}, ETA: {eta_precise}]",
            ));
            bar.set_draw_rate(10);
            Some(bar)
        } else {
            None
        };

        let animation_outputs = self
            .animation_outputs
            .iter()
            .map(|anim| anim.build())
            .collect::<Result<_, _>>()?;

        // TODO: Avoid copying the topology every which way.  If I can
        // wrangle the lifetimes, should be able to have the portal
        // HashMap live in the stage and be borrowed from there.
        Ok(GrowthImage {
            topology: self.topology.clone(),
            pixels,
            stats,
            epsilon: self.epsilon,
            stages,
            active_stage: None,
            current_stage_iter: 0,
            point_tracker: PointTracker::new(self.topology.clone()),
            is_done: false,
            num_filled_pixels: 0,
            rng,
            progress_bar,
            animation_outputs,
        })
    }
}

pub struct GrowthImageStageBuilder {
    palette: Box<dyn Palette>,
    n_colors: Option<u32>,

    max_iter: Option<usize>,

    // For these four, track whether the user explicitly requested
    // specific options for the seed points.  To minimize
    // configuration needed, the first stage and any stages without
    // "grow_from_previous" have 1 random seed point, unless the user
    // explicitly gave seed points, or turned off the random seed
    // points.
    num_random_seed_points: Option<u32>,
    selected_seed_points: Option<Vec<PixelLoc>>,
    grow_from_previous: Option<bool>,
    is_first_stage: bool,

    forbidden_points: Vec<PixelLoc>,
    connected_points: Vec<(PixelLoc, PixelLoc)>,
}

impl GrowthImageStageBuilder {
    fn new(stage_i: usize) -> Self {
        Self {
            palette: Box::new(UniformPalette),
            n_colors: None,
            max_iter: None,
            num_random_seed_points: None,
            selected_seed_points: None,
            grow_from_previous: None,
            is_first_stage: stage_i == 0,
            forbidden_points: Vec::new(),
            connected_points: Vec::new(),
        }
    }

    pub fn palette<T>(&mut self, palette: T) -> &mut Self
    where
        T: Palette + Sized + 'static,
    {
        self.palette = Box::new(palette);
        self
    }

    pub fn n_colors(&mut self, n_colors: u32) -> &mut Self {
        self.n_colors = Some(n_colors);
        self
    }

    pub fn max_iter(&mut self, max_iter: usize) -> &mut Self {
        self.max_iter = Some(max_iter);
        self
    }

    pub fn num_random_seed_points(
        &mut self,
        num_seed_points: u32,
    ) -> &mut Self {
        self.num_random_seed_points = Some(num_seed_points);
        self
    }

    pub fn seed_points(&mut self, seed_points: Vec<PixelLoc>) -> &mut Self {
        self.selected_seed_points = Some(seed_points);
        self
    }

    pub fn grow_from_previous(
        &mut self,
        grow_from_previous: bool,
    ) -> &mut Self {
        self.grow_from_previous = Some(grow_from_previous);
        self
    }

    pub fn forbidden_points(
        &mut self,
        forbidden_points: Vec<PixelLoc>,
    ) -> &mut Self {
        self.forbidden_points = forbidden_points;
        self
    }

    pub fn connected_points(
        &mut self,
        connected_points: Vec<(PixelLoc, PixelLoc)>,
    ) -> &mut Self {
        self.connected_points = connected_points;
        self
    }

    fn build(
        &self,
        topology: &Topology,
        rng: &mut impl Rng,
    ) -> GrowthImageStage {
        let num_random_seed_points = match self.num_random_seed_points {
            Some(n) => n,
            None => {
                if self.selected_seed_points.is_some() {
                    0
                } else if self.is_first_stage
                    || self.grow_from_previous == Some(false)
                {
                    1
                } else {
                    0
                }
            }
        };

        let selected_seed_points = match self.selected_seed_points.as_ref() {
            Some(points) => points.clone(),
            None => Vec::new(),
        };

        let portals = self
            .connected_points
            .iter()
            .filter(|(a, b)| topology.is_valid(*a) && topology.is_valid(*b))
            .flat_map(|&(a, b)| vec![(a, b), (b, a)].into_iter())
            .collect();

        let n_colors = self.n_colors.unwrap_or(topology.len() as u32);
        let palette = KDTree::new(self.palette.generate(n_colors, rng));

        GrowthImageStage {
            palette: palette,
            max_iter: self.max_iter,
            grow_from_previous: self.grow_from_previous.unwrap_or(true),
            selected_seed_points,
            num_random_seed_points,
            forbidden_points: self.forbidden_points.clone(),
            portals,
        }
    }
}

pub struct GrowthImageAnimationBuilder {
    output_file: PathBuf,
    fps: f64,
    iter_per_second: f64,
    layer: u8,
    image_type: SaveImageType,
}

impl GrowthImageAnimationBuilder {
    fn new(output_file: PathBuf) -> Self {
        Self {
            output_file,
            fps: 24.0,
            iter_per_second: 240000.0,
            layer: 0,
            image_type: SaveImageType::Generated,
        }
    }

    pub fn fps(&mut self, fps: f64) -> &mut Self {
        self.fps = fps;
        self
    }

    pub fn iter_per_second(&mut self, iter_per_second: f64) -> &mut Self {
        self.iter_per_second = iter_per_second;
        self
    }

    pub fn layer(&mut self, layer: u8) -> &mut Self {
        self.layer = layer;
        self
    }

    pub fn image_type(&mut self, image_type: SaveImageType) -> &mut Self {
        self.image_type = image_type;
        self
    }

    fn build(&self) -> Result<GrowthImageAnimation, Error> {
        let proc = std::process::Command::new("ffmpeg")
            .args(&["-f", "image2pipe", "-i", "-"])
            .args(&["-hide_banner", "-loglevel", "error"])
            .args(&["-framerate", &self.fps.to_string()])
            .args(&["-vcodec", "libx264"])
            .args(&["-pix_fmt", "yuv420p"])
            // crf for libx264 is on scale from 0 to 51.  0 is lossless.
            .args(&["-crf", "23"])
            .args(&["-preset", "fast"])
            .arg("-y")
            .arg(&self.output_file)
            // Images will be sent to ffmpeg by stdin
            .stdin(std::process::Stdio::piped())
            .spawn()?;

        // TODO: Start ffmpeg subprocess here.
        Ok(GrowthImageAnimation {
            proc,
            iter_per_frame: (self.iter_per_second / self.fps) as usize,
            image_type: self.image_type,
            layer: self.layer,
        })
    }
}
