use std::collections::HashMap;
use std::path::PathBuf;

use itertools::Itertools;
use roxmltree::Document;
use structopt::StructOpt;

use kurbo::{BezPath, ParamCurve, ParamCurveNearest, Shape};

use omnicolor_rust::{
    Error, GrowthImageBuilder, PixelLoc, SaveImageType, SphericalPalette, RGB,
};

use omnicolor_rust::bezier_util::BezPathExt;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(short = "o", long, required_unless_one(&["output-animation", "output-animation-palette"]))]
    output: Option<PathBuf>,

    #[structopt(long)]
    output_animation: Option<PathBuf>,

    #[structopt(long)]
    output_animation_palette: Option<PathBuf>,

    #[structopt(short, long, default_value = "1920")]
    width: u32,

    #[structopt(short, long, default_value = "1080")]
    height: u32,

    #[structopt(long, default_value = "ff6680")]
    first_color: RGB,

    #[structopt(long, default_value = "222222")]
    outline_color: RGB,

    #[structopt(long, default_value = "500000")]
    num_points_outline: usize,

    #[structopt(long, default_value = "80ff66")]
    second_color: RGB,

    #[structopt(long, default_value = "50")]
    color_radius: f32,

    #[structopt(
        long,
        default_value = "50.0",
        help = "Thickness of the rope, in pixels"
    )]
    rope_thickness: f64,

    #[structopt(
        long,
        default_value = "0.8",
        help = "Size of the knot, proportional to the size of the image"
    )]
    knot_size: f64,
}

struct CelticKnotDetails {
    exterior_points_mainlayer: Vec<PixelLoc>,
    exterior_points_underlayer: Vec<PixelLoc>,
    forbidden_points_outline: Vec<PixelLoc>,
    connected_points: Vec<(PixelLoc, PixelLoc)>,
}

fn distance_map_path(
    width: u32,
    height: u32,
    path: &BezPath,
) -> HashMap<PixelLoc, f64> {
    (0..width)
        .cartesian_product(0..height)
        .map(|(i, j)| PixelLoc {
            layer: 0,
            i: i as i32,
            j: j as i32,
        })
        .map(|loc| {
            let point = kurbo::Point::new(loc.i as f64, loc.j as f64);
            let distance = path
                .segments()
                .map(|seg| seg.nearest(point, 0.5))
                .map(|nearest| nearest.distance_sq)
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
                .sqrt();
            (loc, distance)
        })
        .collect()
}

fn distance_map_points(
    width: u32,
    height: u32,
    points: &Vec<kurbo::Point>,
) -> HashMap<PixelLoc, f64> {
    (0..width)
        .cartesian_product(0..height)
        .map(|(i, j)| PixelLoc {
            layer: 0,
            i: i as i32,
            j: j as i32,
        })
        .map(|loc| {
            let point = kurbo::Point::new(loc.i as f64, loc.j as f64);
            let distance = points
                .iter()
                .map(|p| p.distance(point))
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            (loc, distance)
        })
        .collect()
}

#[allow(dead_code)]
struct PixelLocInfo {
    loc: PixelLoc,
    path_distance: f64,
    over_distance: f64,
    under_distance: f64,
    intersection_distance: f64,
    anti_intersection_distance: f64,
}

fn parse_celtic_knot(opt: &Options) -> CelticKnotDetails {
    // Read the path of the knot from file
    let svg_text =
        std::fs::read_to_string("Celtic-knot-basic-linear.svg").unwrap();
    let doc = Document::parse(&svg_text).unwrap();

    let knotpath_text = doc
        .descendants()
        .find(|n| n.attribute("id") == Some("Knotpath"))
        .unwrap()
        .attribute("d")
        .unwrap();

    let mut knotpath = kurbo::BezPath::from_svg(&knotpath_text).unwrap();

    // Scale the path to fill most of the image
    let bbox = knotpath.bounding_box();
    let scale = f64::min(
        (opt.width as f64) / (bbox.x1 - bbox.x0),
        (opt.height as f64) / (bbox.y1 - bbox.y0),
    ) * opt.knot_size;

    knotpath.apply_affine(kurbo::Affine::translate((
        -bbox.center().x,
        -bbox.center().y,
    )));
    knotpath.apply_affine(kurbo::Affine::scale(scale));
    knotpath.apply_affine(kurbo::Affine::translate((
        (opt.width as f64) / 2.0,
        (opt.height as f64) / 2.0,
    )));

    // Break up the path into subpaths whose start and ends are
    // halfway between intersection points.
    let (subpaths, intersections) =
        knotpath.divide_between_intersections(&knotpath);

    // Record the points on the path that are furthest from any
    // intersection.
    let anti_intersections = subpaths
        .iter()
        .map(|seg| seg.segments().next().unwrap().eval(0.0))
        .collect::<Vec<_>>();

    // Group the subpaths into ones that are on top and on bottom at
    // each intersection.
    let (a, b): (Vec<_>, Vec<_>) = subpaths
        .into_iter()
        .enumerate()
        .partition(|(i, _p)| i % 2 == 0);
    let mut groups = vec![a, b].into_iter().map(|paths| {
        BezPath::from_path_segments(
            paths.iter().flat_map(|(_i, path)| path.segments()),
        )
    });
    let over_path = groups.next().unwrap();
    let under_path = groups.next().unwrap();

    // Find the distances from each pixel to critical parts of the
    // path.

    let path_distance = distance_map_path(opt.width, opt.height, &knotpath);
    let over_distance = distance_map_path(opt.width, opt.height, &over_path);
    let under_distance = distance_map_path(opt.width, opt.height, &under_path);
    let intersection_distance =
        distance_map_points(opt.width, opt.height, &intersections);
    let anti_intersection_distance =
        distance_map_points(opt.width, opt.height, &anti_intersections);
    let loc_info = {
        path_distance
            .iter()
            .map(|(&loc, &path_distance)| {
                let over_distance = *over_distance.get(&loc).unwrap();
                let under_distance = *under_distance.get(&loc).unwrap();
                let intersection_distance =
                    *intersection_distance.get(&loc).unwrap();
                let anti_intersection_distance =
                    *anti_intersection_distance.get(&loc).unwrap();
                PixelLocInfo {
                    loc,
                    path_distance,
                    over_distance,
                    under_distance,
                    intersection_distance,
                    anti_intersection_distance,
                }
            })
            .collect::<Vec<_>>()
    };

    // List all points outside the knot
    let exterior_points_mainlayer = loc_info
        .iter()
        .filter(|info| info.path_distance > opt.rope_thickness)
        .map(|info| info.loc)
        .collect::<Vec<_>>();

    // List all points outside the allowed region on the underlayer.
    // Only regions that are needed for the crossovers are enabled, to
    // save on computation time.
    let exterior_points_underlayer = loc_info
        .iter()
        .filter(|info| {
            !((info.path_distance < opt.rope_thickness)
                && (info.intersection_distance
                    < info.anti_intersection_distance)
                && (info.over_distance < opt.rope_thickness * 1.1))
        })
        .map(|info| PixelLoc {
            layer: 1,
            ..info.loc
        })
        .collect::<Vec<_>>();

    // The connections between the main layer and the underlayer.
    // These are on the path, just outside of the intersections.
    let connected_points = loc_info
        .iter()
        .filter(|info| {
            (info.path_distance < opt.rope_thickness)
                && (info.over_distance > opt.rope_thickness * 1.05)
                && (info.over_distance < opt.rope_thickness * 1.1)
                && (info.intersection_distance
                    < info.anti_intersection_distance)
        })
        .map(|info| {
            (
                info.loc,
                PixelLoc {
                    layer: 1,
                    ..info.loc
                },
            )
        })
        .collect::<Vec<_>>();

    // The barriers to prevent the over and under layers from
    // interacting at an intersection.
    let forbidden_points_outline = loc_info
        .iter()
        .filter(|info| {
            (info.path_distance < opt.rope_thickness)
                && (info.over_distance > opt.rope_thickness * 1.0)
                && (info.over_distance < opt.rope_thickness * 1.05)
                && (info.intersection_distance
                    < info.anti_intersection_distance)
        })
        .map(|info| info.loc)
        .collect::<Vec<_>>();

    CelticKnotDetails {
        exterior_points_mainlayer,
        exterior_points_underlayer,
        forbidden_points_outline,
        connected_points,
    }
}

fn main() -> Result<(), Error> {
    let opt = Options::from_args();

    let knot_details = parse_celtic_knot(&opt);

    // Define the builder, with main layer (0) and underlayer (1).
    let mut builder = GrowthImageBuilder::new();
    builder
        .show_progress_bar()
        .epsilon(5.0)
        .add_layer(opt.width, opt.height)
        .add_layer(opt.width, opt.height);

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

    // First stage.  Everything outside the knot is forbidden on the
    // main layer, portals to the underlayer are enabled.
    builder
        .new_stage()
        .palette(SphericalPalette {
            central_color: opt.first_color,
            color_radius: opt.color_radius,
        })
        //.num_random_seed_points(5)
        .connected_points(knot_details.connected_points)
        .forbidden_points(
            knot_details
                .exterior_points_underlayer
                .iter()
                .chain(knot_details.exterior_points_mainlayer.iter())
                .chain(knot_details.forbidden_points_outline.iter())
                .map(|x| *x)
                .collect(),
        );

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
        .forbidden_points(knot_details.exterior_points_underlayer.clone());

    // Last stage.  Allow growth anywhere on the main layer.
    builder
        .new_stage()
        .palette(SphericalPalette {
            central_color: opt.second_color,
            color_radius: opt.color_radius,
        })
        .forbidden_points(knot_details.exterior_points_underlayer);

    // Run the builder.
    let mut image = builder.build()?;
    image.fill_until_done();

    if let Some(output) = opt.output {
        image.write(output);
    }

    Ok(())
}
