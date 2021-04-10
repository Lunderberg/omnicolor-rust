use indicatif::ProgressIterator;

mod growth_image;
mod kd_tree;
mod point_tracker;

use growth_image::{generate_uniform_palette, GrowthImage};

fn main() {
    let width = 1920u32;
    let height = 1080u32;

    let palette = generate_uniform_palette(height * width);
    let mut image = GrowthImage::new(width, height, palette);

    for _i in (0..height * width).progress() {
        //for _i in (0..height * width) {
        image.fill();
    }

    image.write("temp.png");
}
