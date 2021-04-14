use itertools::Itertools;

use crate::kd_tree;
use kd_tree::KDTree;

use crate::errors::Error;

use crate::point_tracker::PointTracker;

#[derive(Debug, Clone, Copy)]
pub struct RGB {
    vals: [u8; 3],
}

impl RGB {
    pub fn r(&self) -> u8 {
        self.vals[0]
    }
    pub fn g(&self) -> u8 {
        self.vals[1]
    }
    pub fn b(&self) -> u8 {
        self.vals[2]
    }
}

impl kd_tree::Point for RGB {
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

pub fn generate_uniform_palette(n_colors: u32) -> Vec<RGB> {
    let mut output = Vec::new();
    output.reserve(n_colors as usize);

    let dim_size = (n_colors as f32).powf(1.0 / 3.0);
    for i in 0..n_colors {
        let val = (i as f32) / dim_size;
        let r = 255.0 * (val % 1.0);
        let val = val.floor() / dim_size;
        let g = 255.0 * (val % 1.0);
        let val = val.floor() / dim_size;
        let b = 255.0 * val;

        output.push(RGB {
            vals: [r as u8, g as u8, b as u8],
        });
    }

    return output;
}

pub struct GrowthImageBuilder {
    width: u32,
    height: u32,
    epsilon: f32,
    palette: Option<Vec<RGB>>,
}

impl GrowthImageBuilder {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
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

    pub fn palette_generator<F>(self, gen: F) -> Self
    where
        F: FnOnce(u32) -> Vec<RGB>,
    {
        let palette = gen(self.width * self.height);
        self.palette(palette)
    }

    pub fn build(self) -> Result<GrowthImage, Error> {
        let palette = self.palette.ok_or(Error::NoPaletteDefined)?;
        let pixels = vec![None; (self.width as usize) * (self.height as usize)];
        let palette = KDTree::new(palette, self.epsilon);
        Ok(GrowthImage {
            width: self.width,
            height: self.height,
            pixels,
            palette,
            point_tracker: PointTracker::new(self.width, self.height),
            done: false,
        })
    }
}

pub struct GrowthImage {
    width: u32,
    height: u32,

    pixels: Vec<Option<RGB>>,
    palette: KDTree<RGB>,
    point_tracker: PointTracker,

    pub done: bool,
}

impl GrowthImage {
    fn get_index(&self, i: i32, j: i32) -> Option<usize> {
        if (i >= 0)
            && (j >= 0)
            && (i < self.width as i32)
            && (j < self.height as i32)
        {
            Some((j as usize) * (self.width as usize) + (i as usize))
        } else {
            None
        }
    }

    fn get_xy(&self, index: usize) -> Option<(u32, u32)> {
        if index < self.pixels.len() {
            Some((
                (index % (self.width as usize)) as u32,
                (index / (self.width as usize)) as u32,
            ))
        } else {
            None
        }
    }

    pub fn fill(&mut self) {
        let res = self.try_fill();
        self.done = res.is_none();
    }

    pub fn get_adjacent_color(&self, i: u32, j: u32) -> Option<RGB> {
        let i = i as i32;
        let j = j as i32;

        let (count, rsum, gsum, bsum) = (-1..=1)
            .cartesian_product(-1..=1)
            .filter(|(di, dj)| (*di != 0) || (*dj != 0))
            .flat_map(|(di, dj)| self.get_index(i + di, j + dj))
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

    fn try_fill(&mut self) -> Option<(u32, u32, RGB)> {
        // No frontier, everything full
        if self.point_tracker.done {
            return None;
        }

        // No frontier, everything empty
        if self.point_tracker.frontier_size() == 0 {
            let i = (self.width as f32 * rand::random::<f32>()) as u32;
            let j = (self.height as f32 * rand::random::<f32>()) as u32;
            self.point_tracker.add_to_frontier(i, j);
        }

        let point_tracker_index = (self.point_tracker.frontier_size() as f32
            * rand::random::<f32>()) as usize;
        let (x, y) = self.point_tracker.get_frontier_point(point_tracker_index);
        self.point_tracker.fill(x, y);

        let next_index = self.get_index(x as i32, y as i32)?;

        let target_color =
            self.get_adjacent_color(x, y).unwrap_or_else(|| RGB {
                vals: [
                    rand::random::<u8>(),
                    rand::random::<u8>(),
                    rand::random::<u8>(),
                ],
            });

        let next_color = self.palette.pop_closest(&target_color)?;

        self.pixels[next_index] = Some(next_color);

        Some((x, y, next_color))
    }

    pub fn write(&self, filename: &str) {
        let file = std::fs::File::create(filename).unwrap();
        let bufwriter = &mut std::io::BufWriter::new(file);

        let mut encoder = png::Encoder::new(bufwriter, self.width, self.height);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        let data = self
            .pixels
            .iter()
            .map(|p| match p {
                Some(rgb) => vec![rgb.r(), rgb.g(), rgb.b(), 255],
                None => vec![0, 0, 0, 0],
            })
            .flat_map(|p| p.into_iter())
            .collect::<Vec<u8>>();
        writer.write_image_data(&data).unwrap();
    }
}
