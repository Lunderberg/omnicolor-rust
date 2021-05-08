use std::path::PathBuf;

use clap::arg_enum;
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

    #[structopt(short, long)]
    seed: Option<u64>,

    #[structopt(long)]
    output_stats: Option<PathBuf>,

    #[structopt(short, long, default_value = "1920")]
    width: u32,

    #[structopt(short, long, default_value = "1080")]
    height: u32,

    #[structopt(short, long, default_value = "5.0")]
    epsilon: f64,

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

    let mut builder = GrowthImageBuilder::new();
    builder
        .show_progress_bar()
        .add_layer(opt.width, opt.height)
        .epsilon(opt.epsilon);
    match opt.palette {
        PaletteOpt::Uniform => builder.palette(UniformPalette),
        PaletteOpt::Spherical => builder.palette(SphericalPalette {
            central_color: opt.central_color.unwrap(),
            color_radius: opt.color_radius.unwrap(),
        }),
    };
    if let Some(seed) = opt.seed {
        builder.seed(seed);
    }

    let mut image = builder.build()?;
    image.fill_until_done();

    if let Some(output) = opt.output {
        image.write(&output);
    }
    if let Some(output) = opt.output_stats {
        image.write_stats(&output);
    }

    Ok(())
}
