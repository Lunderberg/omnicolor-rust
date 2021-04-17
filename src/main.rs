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
                .takes_value(true)
        )
        .arg(
            clap::Arg::with_name("output_stats")
                .short("s")
                .long("output-stats")
                .value_name("FILE")
                .help("Output png file for kd-tree stats")
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

    let output_image = matches.value_of("output");

    let output_stats_image = matches.value_of("output_stats");

    if output_image.is_none() && output_stats_image.is_none() {
        return Err(Error::ArgumentError(
            "Must define at least one of output or output_stats".to_string(),
        ));
    }

    let mut image = GrowthImageBuilder::new(width, height)
        .epsilon(epsilon)
        .palette_generator(generate_uniform_palette)
        .build()?;

    let bar = ProgressBar::new((width * height).into());
    bar.set_style(ProgressStyle::default_bar().template(
        "[{pos}/{len}] {wide_bar} [{elapsed_precise}, ETA: {eta_precise}]",
    ));
    bar.set_draw_rate(10);
    while !image.done {
        image.fill();
        bar.inc(1);
    }
    bar.finish();

    if let Some(output) = output_image {
        image.write(output);
    }
    if let Some(output) = output_stats_image {
        image.write_stats(output);
    }

    Ok(())
}
