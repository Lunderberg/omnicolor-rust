use clap;
use indicatif::ProgressIterator;

mod growth_image;
mod kd_tree;
mod point_tracker;

use growth_image::{generate_uniform_palette, GrowthImage};

fn main() {
    let matches = clap::App::new("omnicolor-rust")
        .about("Generates images using each rgb color exactly once")
        .arg(
            clap::Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Output png file")
                .required(true)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("width")
                .short("w")
                .long("width")
                .value_name("WIDTH")
                .help("Width of the output image")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("height")
                .short("h")
                .long("height")
                .value_name("HEIGHT")
                .help("Height of the output image")
                .takes_value(true),
        )
        .get_matches();

    let width = matches
        .value_of("width")
        .map(|s| s.parse::<u32>().unwrap())
        .unwrap_or(1920u32);

    let height = matches
        .value_of("height")
        .map(|s| s.parse::<u32>().unwrap())
        .unwrap_or(1080u32);

    let output = matches.value_of("output").unwrap();

    let palette = generate_uniform_palette(height * width);
    let mut image = GrowthImage::new(width, height, palette);

    for _i in (0..height * width).progress() {
        image.fill();
    }

    image.write(output);
}
