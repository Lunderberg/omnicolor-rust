use crate::color::RGB;
use crate::common::{PixelLoc, RectangularArray};
use crate::errors::Error;
use crate::growth_image::{GrowthImage, GrowthImageStage};
use crate::kd_tree::KDTree;
use crate::point_tracker::PointTracker;

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

    pub fn new_stage(&mut self) -> &mut GrowthImageStageBuilder {
        let new_stage = GrowthImageStageBuilder::new(self.stages.len());
        self.stages.push(new_stage);
        self.stages.last_mut().unwrap()
    }

    pub fn epsilon(mut self, epsilon: f64) -> Self {
        self.epsilon = epsilon;
        self
    }

    pub fn palette(mut self, palette: Vec<RGB>) -> Self {
        self.new_stage().palette(palette);
        self
    }

    pub fn build(self) -> Result<GrowthImage, Error> {
        if self.stages.len() == 0 {
            return Err(Error::NoStagesDefined);
        }

        let pixels = vec![None; self.size.len()];
        let stats = vec![None; self.size.len()];
        let stages = self.stages.into_iter().map(|s| s.build()).collect();
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
        })
    }
}

pub struct GrowthImageStageBuilder {
    palette: Vec<RGB>,
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
            palette: Vec::new(),
            max_iter: None,
            num_random_seed_points: None,
            selected_seed_points: None,
            grow_from_previous: None,
            is_first_stage: stage_i == 0,
            forbidden_points: Vec::new(),
        }
    }

    pub fn palette(&mut self, palette: Vec<RGB>) -> &mut Self {
        self.palette = palette;
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

    fn build(self) -> GrowthImageStage {
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

        let selected_seed_points =
            self.selected_seed_points.unwrap_or_else(Vec::new);

        GrowthImageStage {
            palette: KDTree::new(self.palette),
            max_iter: self.max_iter,
            grow_from_previous: self.grow_from_previous.unwrap_or(true),
            selected_seed_points,
            num_random_seed_points,
            forbidden_points: self.forbidden_points,
        }
    }
}
