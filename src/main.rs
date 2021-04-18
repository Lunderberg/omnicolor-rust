use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};
use structopt::StructOpt;

mod errors;
mod growth_image;
mod kd_tree;
mod point_tracker;

use errors::Error;
use growth_image::{generate_uniform_palette, GrowthImageBuilder};

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(short = "o", long, required_unless("output-stats"))]
    output: Option<PathBuf>,

    #[structopt(long)]
    output_stats: Option<PathBuf>,

    #[structopt(short, long, default_value = "1920")]
    width: u32,

    #[structopt(short, long, default_value = "1080")]
    height: u32,

    #[structopt(short, long, default_value = "5.0")]
    epsilon: f32,
}

fn main() -> Result<(), Error> {
    let opt = Options::from_args();

    let mut image = GrowthImageBuilder::new(opt.width, opt.height)
        .epsilon(opt.epsilon)
        .palette_generator(generate_uniform_palette)
        .build()?;

    let bar = ProgressBar::new((opt.width * opt.height).into());
    bar.set_style(ProgressStyle::default_bar().template(
        "[{pos}/{len}] {wide_bar} [{elapsed_precise}, ETA: {eta_precise}]",
    ));
    bar.set_draw_rate(10);
    while !image.done {
        image.fill();
        bar.inc(1);
    }
    bar.finish();

    if let Some(output) = opt.output {
        image.write(&output);
    }
    if let Some(output) = opt.output_stats {
        image.write_stats(&output);
    }

    Ok(())
}
