use itertools::Itertools;
use kurbo::{
    BezPath, Line, ParamCurve, ParamCurveArclen, ParamCurveNearest, PathSeg,
    Point, Shape,
};

pub trait BezPathExt {
    fn divide_at_intersections(
        &self,
        other: &BezPath,
    ) -> (Vec<BezPath>, Vec<Point>);
    fn divide_between_intersections(
        &self,
        other: &BezPath,
    ) -> (Vec<BezPath>, Vec<Point>);
    fn as_flat(&self, tolerance: f64) -> BezPath;
    fn subsegment(&self, t: f64) -> (BezPath, BezPath);

    fn regions(&self) -> Vec<BezPath>;

    fn contains_by_intersection_count(&self, point: Point) -> bool;
    fn distance_to_nearest(&self, point: Point) -> f64;
}

impl BezPathExt for BezPath {
    fn divide_at_intersections(
        &self,
        other: &BezPath,
    ) -> (Vec<BezPath>, Vec<Point>) {
        let min_distance_adjacent = 5.0;

        let mut output_sections: Vec<BezPath> = Vec::new();
        let mut output_points: Vec<Point> = Vec::new();

        let mut current: Vec<PathSeg> = Vec::new();

        // Called for each potential intersection.  Makes sure the
        // path length is non-trivial to avoid spurious intersections.
        let mut flush = |current: &mut Vec<PathSeg>, is_last: bool| {
            let pathlen =
                current.iter().map(|seg| seg.arclen(1e-3)).sum::<f64>();
            if pathlen > min_distance_adjacent {
                let completed = std::mem::replace(current, Vec::new());
                if is_last {
                    output_sections[0] = BezPath::from_path_segments(
                        completed
                            .into_iter()
                            .chain(output_sections[0].segments()),
                    );
                } else {
                    output_points.push(completed.last().unwrap().eval(1.0));
                    let path =
                        BezPath::from_path_segments(completed.into_iter());
                    output_sections.push(path);
                }
            }
        };

        self.segments().for_each(|seg| {
            // Exclude intersections from the segment itself, or from
            // adjacent segments, in the case of looking for
            // self-intersections.  Could cause missed intersections
            // that occur directly at boundary between segments.
            let split_by =
                BezPath::from_path_segments(other.segments().filter(|&os| {
                    (os != seg)
                        && (os.start() != seg.end())
                        && (os.end() != seg.start())
                }))
                .as_flat(0.25);

            // List of intersections with this particular segment.
            let mut t_list: Vec<_> = split_by
                .segments()
                .flat_map(|line| {
                    if let PathSeg::Line(line) = line {
                        return seg.intersect_line(line);
                    }
                    panic!();
                })
                .map(|intersection| intersection.segment_t)
                .collect();
            t_list.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

            // Push either segment or subsegment to the current chunk.
            if t_list.is_empty() {
                current.push(seg);
            } else {
                current.push(seg.subsegment(0.0..*t_list.first().unwrap()));
                flush(&mut current, false);

                t_list.iter().tuple_windows().for_each(|(&t1, &t2)| {
                    current.push(seg.subsegment(t1..t2));
                    flush(&mut current, false);
                });
                current.push(seg.subsegment(*t_list.last().unwrap()..1.0));
            }
        });

        flush(&mut current, true);
        (output_sections, output_points)
    }

    fn divide_between_intersections(
        &self,
        other: &BezPath,
    ) -> (Vec<BezPath>, Vec<Point>) {
        let (subpaths, intersections) = self.divide_at_intersections(other);
        let path_halves: Vec<_> = subpaths
            .into_iter()
            .flat_map(|path| {
                let (a, b) = path.subsegment(0.5);
                vec![a, b].into_iter()
            })
            .collect();

        let mut output: Vec<BezPath> = Vec::new();

        let first = path_halves.first().unwrap().clone();
        path_halves
            .iter()
            .skip(1)
            .tuple_windows()
            .step_by(2)
            .for_each(|(a, b)| {
                output.push(BezPath::from_path_segments(
                    a.segments().chain(b.segments()),
                ));
            });
        let last = path_halves.last().unwrap().clone();
        output.push(BezPath::from_path_segments(
            last.segments().chain(first.segments()),
        ));

        (output, intersections)
    }

    fn as_flat(&self, tolerance: f64) -> BezPath {
        let mut elements = Vec::new();
        self.flatten(tolerance, |pathel| elements.push(pathel));
        BezPath::from_vec(elements)
    }

    fn subsegment(&self, t: f64) -> (BezPath, BezPath) {
        let accuracy = 1e-3;

        let length = self.segments().map(|s| s.arclen(accuracy)).sum::<f64>();
        let target_length = length * t;
        let (split_i, split_seg_a, split_seg_b) = self
            .segments()
            .enumerate()
            .scan(0.0, |state, (i, seg)| {
                let length_pre = *state;
                *state += seg.arclen(accuracy);
                let length_post = *state;
                Some((length_pre, i, seg, length_post))
            })
            .filter(|(_, _, _, length_post)| length_post >= &target_length)
            .next()
            .map(|(length_pre, i, seg, _)| {
                let t = seg.inv_arclen(target_length - length_pre, accuracy);
                (i, seg.subsegment(0.0..t), seg.subsegment(t..1.0))
            })
            .unwrap();

        (
            BezPath::from_path_segments(
                self.segments()
                    .take(split_i)
                    .chain(std::iter::once(split_seg_a)),
            ),
            BezPath::from_path_segments(
                std::iter::once(split_seg_b)
                    .chain(self.segments().skip(split_i + 1)),
            ),
        )
    }

    fn regions(&self) -> Vec<BezPath> {
        self.elements()
            .split_inclusive(|&pathel| pathel == kurbo::PathEl::ClosePath)
            .map(|elements| elements.iter().map(|x| *x).collect())
            .collect()
    }

    fn contains_by_intersection_count(&self, point: Point) -> bool {
        let bbox = self.bounding_box();
        if bbox.contains(point) {
            let outside_point = Point::new(bbox.min_x() - 1.0, bbox.min_y());
            let line = Line::new(point, outside_point);
            let num_intersections = self
                .segments()
                .map(|seg| seg.intersect_line(line).len())
                .sum::<usize>();
            num_intersections % 2 != 0
        } else {
            false
        }
    }

    fn distance_to_nearest(&self, point: Point) -> f64 {
        self.segments()
            .map(|seg| seg.nearest(point, 1e-3).distance_sq)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
            .sqrt()
    }
}
