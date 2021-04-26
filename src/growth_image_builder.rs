use rand::{Rng, SeedableRng};

use crate::common::{PixelLoc, RectangularArray};
use crate::errors::Error;
use crate::growth_image::{GrowthImage, GrowthImageStage};
use crate::kd_tree::KDTree;
use crate::palettes::{Palette, UniformPalette};
use crate::point_tracker::PointTracker;

pub struct GrowthImageBuilder {
    size: RectangularArray,
    epsilon: f64,
    stages: Vec<GrowthImageStageBuilder>,
    seed: Option<u64>,
}

impl GrowthImageBuilder {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: RectangularArray { width, height },
            epsilon: 1.0,
            stages: Vec::new(),
            seed: None,
        }
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

        let mut rng = match self.seed {
            Some(seed) => rand_chacha::ChaCha8Rng::seed_from_u64(seed),
            None => rand_chacha::ChaCha8Rng::from_entropy(),
        };

        let pixels = vec![None; self.size.len()];
        let stats = vec![None; self.size.len()];
        let stages = self
            .stages
            .iter()
            .map(|s| s.build(&self.size, &mut rng))
            .collect();

        Ok(GrowthImage {
            size: self.size,
            pixels,
            stats,
            epsilon: self.epsilon,
            stages,
            active_stage: None,
            current_stage_iter: 0,
            point_tracker: PointTracker::new(self.size),
            is_done: false,
            num_filled_pixels: 0,
            rng,
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

    fn build(
        &self,
        size: &RectangularArray,
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

        let n_colors = self.n_colors.unwrap_or(size.len() as u32);
        let palette = KDTree::new(self.palette.generate(n_colors, rng));

        GrowthImageStage {
            palette: palette,
            max_iter: self.max_iter,
            grow_from_previous: self.grow_from_previous.unwrap_or(true),
            selected_seed_points,
            num_random_seed_points,
            forbidden_points: self.forbidden_points.clone(),
        }
    }
}
