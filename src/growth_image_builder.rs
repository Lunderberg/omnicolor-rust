use crate::color::RGB;
use crate::common::RectangularArray;
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
        self.stages.push(GrowthImageStageBuilder::new());
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
        Ok(GrowthImage {
            size: self.size,
            pixels,
            stats,
            epsilon: self.epsilon,
            stages: self.stages.into_iter().map(|s| s.build()).collect(),
            active_stage: None,
            current_stage_iter: 0,
            point_tracker: PointTracker::new(self.size),
            is_done: false,
        })
    }
}

pub struct GrowthImageStageBuilder {
    palette: Vec<RGB>,
    max_iter: Option<usize>,
}

impl GrowthImageStageBuilder {
    fn new() -> Self {
        Self {
            palette: Vec::new(),
            max_iter: None,
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

    fn build(self) -> GrowthImageStage {
        GrowthImageStage {
            palette: KDTree::new(self.palette),
            max_iter: self.max_iter,
        }
    }
}
