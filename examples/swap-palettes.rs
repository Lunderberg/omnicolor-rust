use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};
use structopt::StructOpt;

use omnicolor_rust::palettes::generate_spherical_palette;
use omnicolor_rust::{Error, GrowthImageBuilder, GrowthImageStageBuilder, RGB};

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(short = "o", long)]
    output: PathBuf,

    #[structopt(short, long, default_value = "1920")]
    width: u32,

    #[structopt(short, long, default_value = "1080")]
    height: u32,

    #[structopt(short, long, default_value = "0.5")]
    proportion_first_color: f32,

    #[structopt(
        long,
        default_value = "1.0",
        help = "Size of the color palette relative to the number of pixels in each stage"
    )]
    proportion_excess_colors: f32,

    #[structopt(long, default_value = "ff6680")]
    first_color: RGB,

    #[structopt(long, default_value = "80ff66")]
    second_color: RGB,

    #[structopt(long, default_value = "50")]
    color_radius: f32,
}

fn main() -> Result<(), Error> {
    let opt = Options::from_args();

    let num_pixels_first =
        ((opt.width * opt.height) as f32 * opt.proportion_first_color) as usize;
    let num_pixels_second =
        (opt.width * opt.height) as usize - num_pixels_first;

    let num_colors_first =
        ((num_pixels_first as f32) * opt.proportion_excess_colors) as u32;
    let num_colors_second =
        ((num_pixels_second as f32) * opt.proportion_excess_colors) as u32;

    let first_palette = generate_spherical_palette(
        num_colors_first,
        opt.first_color,
        opt.color_radius,
    );
    let second_palette = generate_spherical_palette(
        num_colors_second,
        opt.second_color,
        opt.color_radius,
    );

    let mut image = GrowthImageBuilder::new(opt.width, opt.height)
        .add_stage(GrowthImageStageBuilder {
            palette: first_palette,
            max_iter: Some(num_pixels_first),
        })
        .add_stage(GrowthImageStageBuilder {
            palette: second_palette,
            ..Default::default()
        })
        .epsilon(5.0)
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

    image.write(&opt.output);

    Ok(())
}
