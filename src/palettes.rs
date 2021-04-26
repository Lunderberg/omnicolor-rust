use rand::{Rng, RngCore};

use crate::color::RGB;

pub trait Palette {
    fn generate(&self, n_colors: u32, rng: &mut dyn RngCore) -> Vec<RGB>;
}

#[derive(Copy, Clone)]
pub struct UniformPalette;

impl Palette for UniformPalette {
    fn generate(&self, n_colors: u32, _: &mut dyn RngCore) -> Vec<RGB> {
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

        output
    }
}

#[derive(Copy, Clone)]
pub struct SphericalPalette {
    pub central_color: RGB,
    pub color_radius: f32,
}

impl Palette for SphericalPalette {
    fn generate(&self, n_colors: u32, rng: &mut dyn RngCore) -> Vec<RGB> {
        let mut output = Vec::new();
        output.reserve(n_colors as usize);

        for _i in 0..n_colors {
            let r = self.color_radius * rng.gen::<f32>().powf(1.0 / 3.0);
            let phi = 2.0 * std::f32::consts::PI * rng.gen::<f32>();
            let costheta = 1.0 - 2.0 * rng.gen::<f32>();
            let sintheta = (1.0 - costheta * costheta).sqrt();

            let dx = r * sintheta * phi.cos();
            let dy = r * sintheta * phi.sin();
            let dz = r * costheta;

            let color = RGB {
                vals: [
                    ((self.central_color.r() as f32) + dx).clamp(0.0, 255.0)
                        as u8,
                    ((self.central_color.g() as f32) + dy).clamp(0.0, 255.0)
                        as u8,
                    ((self.central_color.b() as f32) + dz).clamp(0.0, 255.0)
                        as u8,
                ],
            };
            output.push(color);
        }

        output
    }
}
