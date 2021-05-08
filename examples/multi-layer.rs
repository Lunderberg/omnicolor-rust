use std::path::PathBuf;

use structopt::StructOpt;

use omnicolor_rust::palettes::*;
use omnicolor_rust::{Error, GrowthImageBuilder, PixelLoc, RGB};

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(short = "o", long)]
    output: PathBuf,

    #[structopt(short, long, default_value = "1920")]
    width: u32,

    #[structopt(short, long, default_value = "1080")]
    height: u32,

    #[structopt(long, default_value = "ff6680")]
    first_color: RGB,

    #[structopt(long, default_value = "80ff66")]
    second_color: RGB,
}

fn main() -> Result<(), Error> {
    let opt = Options::from_args();

    // Vertical walls at 20% and 80%
    let walls = [opt.width * 2 / 10, opt.width * 8 / 10]
        .iter()
        .map(|&i| {
            (0..opt.height).map(move |j| PixelLoc {
                layer: 0,
                i: i as i32,
                j: j as i32,
            })
        })
        .flatten()
        .collect::<Vec<_>>();

    let bridge1_width = 300i32;
    let bridge1_height = 100i32;
    // Portal to the bridge at the bottom-left
    let portal1 = (0..bridge1_height).map(|j| {
        (
            PixelLoc {
                i: 0,
                j: (opt.height as i32) - j,
                layer: 0,
            },
            PixelLoc {
                i: 0,
                j: j,
                layer: 1,
            },
        )
    });
    // Portal to the bridge at the bottom-right
    let portal2 = (0..bridge1_height).map(|j| {
        (
            PixelLoc {
                i: (opt.width as i32) - 1,
                j: (opt.height as i32) - j,
                layer: 0,
            },
            PixelLoc {
                i: bridge1_width - 1,
                j: j,
                layer: 1,
            },
        )
    });

    let bridge2_width = 500i32;
    let bridge2_height = 100i32;
    // Portal to the bridge at the top-right
    let portal3 = (0..bridge2_height).map(|j| {
        (
            PixelLoc {
                i: (opt.width - 1) as i32,
                j,
                layer: 0,
            },
            PixelLoc {
                i: (bridge2_width - 1) as i32,
                j,
                layer: 2,
            },
        )
    });
    // Portal to the bridge at the top-center
    let portal4 = (0..bridge2_height).map(|j| {
        (
            PixelLoc {
                i: (opt.width / 2) as i32,
                j,
                layer: 0,
            },
            PixelLoc {
                i: 0,
                j: j as i32,
                layer: 2,
            },
        )
    });
    let portals = portal1
        .chain(portal2)
        .chain(portal3)
        .chain(portal4)
        .collect::<Vec<_>>();

    //let num_pixels_first = (opt.width * opt.height / 2) as usize;
    let num_pixels_first = (opt.width * opt.height) as usize;

    let color_radius = 50.0;
    let first_palette = SphericalPalette {
        central_color: opt.first_color,
        color_radius,
    };
    let second_palette = SphericalPalette {
        central_color: opt.second_color,
        color_radius,
    };

    let mut builder = GrowthImageBuilder::new();
    builder
        .show_progress_bar()
        .add_layer(opt.width, opt.height)
        .add_layer(bridge1_width as u32, bridge1_height as u32)
        .add_layer(bridge2_width as u32, bridge2_height as u32)
        .epsilon(5.0);

    builder
        .new_stage()
        .palette(first_palette)
        .max_iter(num_pixels_first)
        .seed_points(vec![PixelLoc {
            i: (opt.width / 10) as i32,
            j: 0,
            layer: 0,
        }])
        .forbidden_points(walls)
        .connected_points(portals);

    builder.new_stage().palette(second_palette);

    let mut image = builder.build()?;
    image.fill_until_done();
    image.write(opt.output);

    Ok(())
}
