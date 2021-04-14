use clap;
use indicatif::{ProgressBar, ProgressStyle};

mod errors;
mod growth_image;
mod kd_tree;
mod point_tracker;

use errors::Error;
use growth_image::{generate_uniform_palette, GrowthImageBuilder};

fn main() -> Result<(), Error> {
    let matches = clap::App::new("omnicolor-rust")
        .about("Generates images using each rgb color exactly once")
        .arg(
            clap::Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Output png file")
                .required(true)
                .takes_value(true)
        )
        .arg(
            clap::Arg::with_name("width")
                .short("w")
                .long("width")
                .value_name("WIDTH")
                .help("Width of the output image")
                .takes_value(true)
        )
        .arg(
            clap::Arg::with_name("height")
                .short("h")
                .long("height")
                .value_name("HEIGHT")
                .help("Height of the output image")
                .takes_value(true)
        )
        .arg(
            clap::Arg::with_name("epsilon")
                .short("e")
                .long("epsilon")
                .value_name("FLOAT")
                .help("Precision parameter, how close to the closest point to look")
                .takes_value(true)
            )
        .get_matches();

    let width = matches
        .value_of("width")
        .map(|s| s.parse::<u32>())
        .unwrap_or(Ok(1920u32))?;

    let height = matches
        .value_of("height")
        .map(|s| s.parse::<u32>())
        .unwrap_or(Ok(1080u32))?;

    let epsilon = matches
        .value_of("epsilon")
        .map(|s| s.parse::<f32>())
        .unwrap_or(Ok(1.0))?;

    let output = matches.value_of("output").unwrap();

    let mut image = GrowthImageBuilder::new(width, height)
        .epsilon(epsilon)
        .palette_generator(generate_uniform_palette)
        .build()?;

    let bar = ProgressBar::new((width * height).into());
    bar.set_style(ProgressStyle::default_bar().template(
        "[{pos}/{len}] {wide_bar} [{elapsed_precise}, ETA: {eta_precise}]",
    ));
    while !image.done {
        image.fill();
        bar.inc(1);
    }
    bar.finish();

    image.write(output);

    Ok(())
}
