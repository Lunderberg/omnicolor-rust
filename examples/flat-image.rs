use std::path::PathBuf;

use clap::arg_enum;
use indicatif::{ProgressBar, ProgressStyle};
use structopt::StructOpt;

use omnicolor_rust::palettes::*;
use omnicolor_rust::{Error, GrowthImageBuilder, RGB};

arg_enum! {
    #[derive(Debug, PartialEq)]
    enum PaletteOpt{
        Uniform,
        Spherical,
    }
}

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

    #[structopt(short, long,
                default_value = "uniform",
                case_insensitive = true,
                possible_values = &PaletteOpt::variants())
    ]
    palette: PaletteOpt,

    #[structopt(long, required_if("palette", "spherical"))]
    central_color: Option<RGB>,

    #[structopt(long, required_if("palette", "spherical"))]
    color_radius: Option<f32>,
}

fn main() -> Result<(), Error> {
    let opt = Options::from_args();

    let palette = match opt.palette {
        PaletteOpt::Uniform => generate_uniform_palette(opt.width * opt.height),
        PaletteOpt::Spherical => generate_spherical_palette(
            opt.width * opt.height,
            opt.central_color.unwrap(),
            opt.color_radius.unwrap(),
        ),
    };

    let mut image = GrowthImageBuilder::new(opt.width, opt.height)
        .epsilon(opt.epsilon)
        .palette(palette)
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
