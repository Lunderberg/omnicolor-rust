use std::path::PathBuf;

use itertools::Itertools;
use kurbo::{BezPath, ParamCurve, Shape};
use structopt::StructOpt;

use omnicolor_rust::bezier_util::BezPathExt;
use omnicolor_rust::{
    Error, GrowthImageBuilder, PixelLoc, SaveImageType, SphericalPalette, RGB,
};

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(short = "o", long, required_unless_one(&["output-animation", "output-animation-palette"]))]
    output: Option<PathBuf>,

    #[structopt(long)]
    output_layer2: Option<PathBuf>,

    #[structopt(long)]
    output_animation: Option<PathBuf>,

    #[structopt(long)]
    output_animation_palette: Option<PathBuf>,

    #[structopt(short, long, default_value = "1920")]
    width: u32,

    #[structopt(short, long, default_value = "1080")]
    height: u32,

    #[structopt(long, default_value = "f5b00f")]
    first_color: RGB,

    #[structopt(long, default_value = "1.0")]
    first_palette_size: f64,

    #[structopt(long, default_value = "222222")]
    outline_color: RGB,

    #[structopt(long, default_value = "100000")]
    num_points_outline: usize,

    #[structopt(long, default_value = "0f3df5")]
    second_color: RGB,

    #[structopt(long, default_value = "50")]
    color_radius: f32,

    #[structopt(
        long,
        default_value = "0.8",
        help = "Size of the logo, proportional to the size of the image"
    )]
    logo_size: f64,
}

struct LogoDetails {
    interior_points: Vec<PixelLoc>,
    underworld_exterior_points: Vec<PixelLoc>,
    connected_points: Vec<(PixelLoc, PixelLoc)>,
    initial_points: Vec<PixelLoc>,
}

struct PointDetails {
    loc: PixelLoc,
    point: kurbo::Point,
}

fn parse_octoml_logo(opt: &Options) -> LogoDetails {
    // Outline of OctoML logo, from https://octoml.ai/img/logo.svg
    let logo_path_text = "
m19.9349 48.1127-1.1899 1.1887c-2.0644 2.0631-4.7878 3.0948-7.5109 3.0948l-.009.009c-2.72278 0-5.44948-1.0353-7.51963-3.1038l-.00072-.0003c-2.06436-2.0631-3.096723-4.7848-3.097084-7.5064v-.0174c.000361-2.2479.704984-4.4962 2.113144-6.3729.50191-.6687 1.19641-1.0468 2.03112-1.1061 2.29237-.1639 4.23423 2.6705 2.72092 4.7158-.6067.8197-.91131 1.7962-.91348 2.7719.00253 1.1942.45819 2.3895 1.36625 3.2969l.00072.0008c.90481.9042 2.10156 1.3566 3.29876 1.3566v.0087c1.1949-.0025 2.3913-.4579 3.2994-1.3653l1.1895-1.1892zm15.347-41.79541.0004.00037c3.6427 3.64036 5.4639 8.43624 5.4642 13.22854-.0003 4.7934-1.8215 9.589-5.4642 13.229l-6.9064 6.9019h-.0004l-4.2201-4.2178h-.0007l7.0151-7.0109c2.4051-2.4621 3.6081-5.6814 3.6081-8.9022 0-3.268-1.2387-6.535-3.7158-9.0104l-.0007-.0007c-2.4774-2.47581-5.7461-3.71333-9.0166-3.71369-3.2705 0-6.5396 1.23788-9.0166 3.71399l-.0008.0004c-2.477 2.4754-3.71603 5.742-3.71603 9.0104 0 3.2374 1.21553 6.4733 3.64593 8.9401l9.0875 9.0819.0004.0004 7.5199 7.5151c.9081.9074 2.1041 1.3631 3.2991 1.3653v-.0087c1.1975 0 2.3939-.4528 3.2994-1.357l.0004-.0004c.9084-.9078 1.364-2.1034 1.3659-3.2969-.0015-.9754-.3065-1.9525-.9132-2.7719-2.1698-2.9304 2.4745-6.6437 4.7517-3.61 1.4082 1.877 2.1128 4.1253 2.1135 6.3732v.0174c-.0007 2.7216-1.0331 5.4426-3.0974 7.5067h-.0007c-2.0698 2.0685-4.7965 3.1038-7.5196 3.1038l-.0083-.0083c-2.7239-.0007-5.4477-1.0324-7.5117-3.0955l-5.4097-5.4058v-.0007l-11.12757-11.1197c-3.64235-3.6404-5.46389-8.4356-5.46425-13.229.00036-4.7923 1.8219-9.58818 5.46425-13.22854l.00037-.00037c3.6427-3.64 8.4414-5.460356 13.2375-5.460356s9.5948 1.819996 13.2371 5.460356z
";

    // Convert from SVG to BezPath, center and scale
    let mut path = BezPath::from_svg(&logo_path_text).unwrap();
    {
        let bbox = path.bounding_box();
        let scale = f64::min(
            (opt.width as f64) / (bbox.x1 - bbox.x0),
            (opt.height as f64) / (bbox.y1 - bbox.y0),
        ) * opt.logo_size;

        path.apply_affine(kurbo::Affine::translate((
            -bbox.center().x,
            -bbox.center().y,
        )));
        path.apply_affine(kurbo::Affine::scale(scale));
        path.apply_affine(kurbo::Affine::translate((
            (opt.width as f64) / 2.0,
            (opt.height as f64) / 2.0,
        )));
    }

    // Utility helper to list all PixelLoc/kurbo::Point combos
    let point_details = (0..opt.width)
        .cartesian_product(0..opt.height)
        .map(|(i, j)| PointDetails {
            loc: PixelLoc {
                layer: 0,
                i: i as i32,
                j: j as i32,
            },
            point: kurbo::Point::new(i as f64, j as f64),
        })
        .collect::<Vec<_>>();

    // Find all points inside the logo, for first stage.
    let mainlayer_interior_points = point_details
        .iter()
        .filter(|d| {
            //https://github.com/linebender/kurbo/issues/180
            //path.contains(point)
            path.contains_by_intersection_count(d.point)
        })
        .map(|d| d.loc)
        .collect::<Vec<_>>();

    // Separate out the lines at the edge of the logo, near the
    // overlap.
    let portal_lines = path
        .regions()
        .iter()
        .enumerate()
        .map(|(i, reg)| match i {
            0 => reg.segments().last().unwrap(),
            1 => reg.segments().skip(5).next().unwrap(),
            _ => panic!("Too many regions, wrong SVG"),
        })
        .collect::<Vec<_>>();

    // Extent of the underworld.  Only the connection itself should be
    // allowed, because otherwise the imagegen visuals would slow down
    // as most updates only affect the underworld.
    let mut underworld_bounds = BezPath::new();
    underworld_bounds.move_to(portal_lines[0].start());
    underworld_bounds.line_to(portal_lines[0].end());
    underworld_bounds.line_to(portal_lines[1].start());
    underworld_bounds.line_to(portal_lines[1].end());
    underworld_bounds.close_path();

    // Define connected points between the main layer and the
    // underworld, along the portal lines.
    let portal_path = BezPath::from_path_segments(portal_lines.into_iter());
    let connected_points = point_details
        .iter()
        .filter(|d| portal_path.distance_to_nearest(d.point) < 5.0)
        .map(|d| (d.loc, PixelLoc { layer: 1, ..d.loc }))
        .collect::<Vec<_>>();

    // The interior of the underworld, as defined by the underworld
    // bounds.  The underworld side of the connected points are
    // explicitly added.  Otherwise, underworld_interior_points and
    // mainlayer_interior_points are separated by a single pixel.  Used for the
    // first stage, to bridge across the disconnected regions in the
    // logo.
    let underworld_interior_points = point_details
        .iter()
        .filter(|d| underworld_bounds.contains_by_intersection_count(d.point))
        .map(|d| PixelLoc { layer: 1, ..d.loc })
        .chain(
            connected_points
                .iter()
                .map(|&(_main, underworld)| underworld),
        )
        .collect::<Vec<_>>();

    let interior_points = mainlayer_interior_points
        .into_iter()
        .chain(underworld_interior_points.into_iter())
        .collect::<Vec<_>>();

    // The exterior of the underworld.  Used to forbid these regions
    // during every stage after the first.
    let underworld_exterior_points = point_details
        .iter()
        .filter(|d| !underworld_bounds.contains_by_intersection_count(d.point))
        .map(|d| PixelLoc { layer: 1, ..d.loc })
        .collect::<Vec<_>>();

    // Find center of circular bit at the bottom left of logo.
    let p_loc_left = {
        let p1 = path.segments().nth(8).unwrap().start();
        let p2 = path.segments().nth(9).unwrap().end();
        let p_mid = p1.midpoint(p2);
        PixelLoc {
            layer: 0,
            i: p_mid.x as i32,
            j: p_mid.y as i32,
        }
    };
    // Reflect to find circular bit at the bottom right of logo.
    let p_loc_right = PixelLoc {
        i: (opt.width as i32) - p_loc_left.i,
        ..p_loc_left
    };
    let initial_points = vec![p_loc_left, p_loc_right];

    LogoDetails {
        interior_points,
        underworld_exterior_points,
        connected_points,
        initial_points,
    }
}

fn main() -> Result<(), Error> {
    let opt = Options::from_args();

    let details = parse_octoml_logo(&opt);

    // Define the builder, with main layer (0) and underlayer (1).
    let mut builder = GrowthImageBuilder::new();
    builder
        .show_progress_bar()
        .epsilon(5.0)
        .add_layer(opt.width, opt.height)
        .add_layer(opt.width, opt.height);

    // First stage.  Everything outside the knot is forbidden on the
    // main layer, portals to the underlayer are enabled.
    let n_colors_first = ((details.interior_points.len() as f64)
        * opt.first_palette_size) as u32;
    builder
        .new_stage()
        .palette(SphericalPalette {
            central_color: opt.first_color,
            color_radius: opt.color_radius,
        })
        .n_colors(n_colors_first)
        .animation_iter_per_second(20000.0)
        .connected_points(details.connected_points)
        .seed_points(details.initial_points)
        .allowed_points(details.interior_points);

    // Outline stage.  Keep forbidding points on the underlayer, but
    // allow growth outside of the knot itself.  Apply a max number of
    // iterations in order to control the size of the border.
    builder
        .new_stage()
        .palette(SphericalPalette {
            central_color: opt.outline_color,
            color_radius: opt.color_radius,
        })
        .max_iter(opt.num_points_outline)
        .forbidden_points(details.underworld_exterior_points.clone());

    // Last stage.  Allow growth anywhere on the main layer.
    builder
        .new_stage()
        .palette(SphericalPalette {
            central_color: opt.second_color,
            color_radius: opt.color_radius,
        })
        .forbidden_points(details.underworld_exterior_points.clone());

    if let Some(output) = opt.output_animation {
        builder
            .add_output_animation(output)
            .image_type(SaveImageType::Generated);
    }

    if let Some(output) = opt.output_animation_palette {
        builder
            .add_output_animation(output)
            .image_type(SaveImageType::ColorPalette);
    }

    // Run the builder.
    let mut image = builder.build()?;
    image.fill_until_done();

    if let Some(output) = opt.output {
        image.write(output);
    }

    if let Some(output) = opt.output_layer2 {
        image.write_image(output, SaveImageType::Generated, 1);
    }

    Ok(())
}
