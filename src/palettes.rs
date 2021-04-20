use crate::color::RGB;

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

    output
}

pub fn generate_spherical_palette(
    n_colors: u32,
    central_color: RGB,
    color_radius: f32,
) -> Vec<RGB> {
    let mut output = Vec::new();
    output.reserve(n_colors as usize);

    for _i in 0..n_colors {
        let r = color_radius * rand::random::<f32>().powf(1.0 / 3.0);
        let phi = 2.0 * std::f32::consts::PI * rand::random::<f32>();
        let costheta = 1.0 - 2.0 * rand::random::<f32>();
        let sintheta = (1.0 - costheta * costheta).sqrt();

        let dx = r * sintheta * phi.cos();
        let dy = r * sintheta * phi.sin();
        let dz = r * costheta;

        let color = RGB {
            vals: [
                ((central_color.r() as f32) + dx).clamp(0.0, 255.0) as u8,
                ((central_color.g() as f32) + dy).clamp(0.0, 255.0) as u8,
                ((central_color.b() as f32) + dz).clamp(0.0, 255.0) as u8,
            ],
        };
        output.push(color);
    }

    output
}
