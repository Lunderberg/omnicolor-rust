const MAX_LEAF_SIZE: usize = 50;

pub trait Point: Copy + std::fmt::Debug {
    type Dtype: PartialOrd + Copy + Into<f64> + std::fmt::Debug;
    const NUM_DIMENSIONS: u8;

    // Dimension parameter guaranteed to be less than NUM_DIMENSIONS.
    fn get_val(&self, dimension: u8) -> Self::Dtype;

    // Returns the distance-squared between current point and other
    // point.
    fn dist2(&self, other: &Self) -> f64;
}

#[derive(Debug)]
enum NodeData<T: Point> {
    Internal {
        left: usize,
        right: usize,
        dimension: u8,
        median_val: T::Dtype,
    },
    Leaf {
        i_initial: usize,
        i_final: usize,
    },
}

#[derive(Debug)]
struct Node<T: Point> {
    num_points: u32,
    parent: Option<usize>,
    data: NodeData<T>,
}

pub struct KDTree<T: Point> {
    points: Vec<Option<T>>,
    nodes: Vec<Node<T>>,
}

#[derive(Clone, Copy)]
struct SearchRes {
    dist2: f64,
    point_index: usize,
    leaf_node_index: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct PerformanceStats {
    pub nodes_checked: u32,
    pub leaf_nodes_checked: u32,
    pub points_checked: u32,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        PerformanceStats {
            nodes_checked: 0,
            leaf_nodes_checked: 0,
            points_checked: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct KdtreeResult<T: Point> {
    pub res: Option<T>,
    pub stats: PerformanceStats,
}

impl<T> KDTree<T>
where
    T: Point,
{
    pub fn new(mut points: Vec<T>) -> Self {
        let mut nodes = Vec::new();

        Self::generate_nodes(&mut nodes, &mut points, 0, 0, None);

        let points = points.iter().map(|p| Some(*p)).collect();

        KDTree { points, nodes }
    }

    pub fn num_points(&self) -> usize {
        self.nodes[0].num_points as usize
    }

    pub fn iter_points(&self) -> impl Iterator<Item = &Option<T>> {
        self.points.iter()
    }

    fn generate_nodes(
        nodes: &mut Vec<Node<T>>,
        points: &mut [T],
        point_index_offset: usize,
        dimension: u8,
        parent_index: Option<usize>,
    ) {
        // If few enough points, make a leaf node.
        if points.len() <= MAX_LEAF_SIZE {
            let node = Node {
                num_points: points.len() as u32,
                parent: parent_index,
                data: NodeData::Leaf {
                    i_initial: point_index_offset,
                    i_final: point_index_offset + points.len(),
                },
            };
            nodes.push(node);
            return;
        }

        let median_point_index = points.len() / 2;
        // Can't use select_nth_unstable_by_key because that requires
        // Ord, which f32/f64 don't implement.  The .unwrap() could
        // panic if passed NaN values.
        points.select_nth_unstable_by(median_point_index, |a, b| {
            a.get_val(dimension)
                .partial_cmp(&b.get_val(dimension))
                .unwrap()
        });
        let median_val = points[median_point_index].get_val(dimension);

        let this_node_index = nodes.len();
        let node = Node {
            parent: parent_index,
            num_points: points.len() as u32,
            data: NodeData::Internal {
                left: this_node_index + 1,
                right: 0, // Will be overwritten once known
                dimension,
                median_val,
            },
        };
        nodes.push(node);

        let next_dimension = (dimension + 1) % T::NUM_DIMENSIONS;

        // Generate the left subtree
        Self::generate_nodes(
            nodes,
            &mut points[..median_point_index],
            point_index_offset,
            next_dimension,
            Some(this_node_index),
        );

        // Now, the index of the right subtree is known and can be
        // updated.
        let right_node_index = nodes.len();
        if let NodeData::Internal { right, .. } =
            &mut nodes[this_node_index].data
        {
            *right = right_node_index;
        }

        // Generate the right subtree
        Self::generate_nodes(
            nodes,
            &mut points[median_point_index..],
            point_index_offset + median_point_index,
            next_dimension,
            Some(this_node_index),
        );
    }

    #[allow(dead_code)]
    pub fn get_closest(&self, target: &T, epsilon: f64) -> KdtreeResult<T> {
        let mut stats = PerformanceStats::default();
        let res = self
            .get_closest_node(target, 0, &mut stats, epsilon)
            .map(|res| self.points[res.point_index])
            .flatten();
        KdtreeResult { res, stats }
    }

    pub fn pop_closest(&mut self, target: &T, epsilon: f64) -> KdtreeResult<T> {
        let mut stats = PerformanceStats::default();
        let res = self.get_closest_node(target, 0, &mut stats, epsilon);

        let res = match res {
            None => None,
            Some(res) => {
                let output = self.points[res.point_index];

                self.points[res.point_index] = None;
                let mut node_index = Some(res.leaf_node_index);
                while node_index != None {
                    let node = &mut self.nodes[node_index.unwrap()];
                    node.num_points -= 1;
                    node_index = node.parent;
                }

                output
            }
        };
        KdtreeResult { res, stats }
    }

    fn get_closest_node(
        &self,
        target: &T,
        node_index: usize,
        stats: &mut PerformanceStats,
        epsilon: f64,
    ) -> Option<SearchRes> {
        let node = &self.nodes[node_index];
        if node.num_points == 0 {
            return None;
        }
        stats.nodes_checked += 1;

        match &node.data {
            NodeData::Leaf { i_initial, i_final } => {
                stats.leaf_nodes_checked += 1;
                stats.points_checked += node.num_points;

                // If it is a leaf node, just check each distance.
                let (point_index, dist2) = (*i_initial..*i_final)
                    .map(|i| (i, self.points[i]))
                    .filter_map(|(i, opt_p)| {
                        opt_p.map(|p| (i, p.dist2(target)))
                    })
                    .min_by(|(_, a_dist2), (_, b_dist2)| {
                        a_dist2.partial_cmp(b_dist2).unwrap()
                    })
                    .unwrap();
                Some(SearchRes {
                    dist2,
                    leaf_node_index: node_index,
                    point_index,
                })
            }

            NodeData::Internal {
                left,
                right,
                dimension,
                median_val,
            } => {
                let diff: f64 =
                    target.get_val(*dimension).into() - (*median_val).into();
                let (search_first, search_second) = if diff < 0.0 {
                    (left, right)
                } else {
                    (right, left)
                };

                // If it is an internal node, start by checking the
                // half that contains the target point.
                let res1 = self.get_closest_node(
                    target,
                    *search_first,
                    stats,
                    epsilon,
                );
                if res1
                    .filter(|r| {
                        let max_dist2 = (diff * (epsilon + 1.0)).powf(2.0);
                        r.dist2 < max_dist2
                    })
                    .is_some()
                {
                    return res1;
                }

                let res2 = self.get_closest_node(
                    target,
                    *search_second,
                    stats,
                    epsilon,
                );

                [res1, res2]
                    .iter()
                    .flatten()
                    .min_by(|a, b| a.dist2.partial_cmp(&b.dist2).unwrap())
                    .map(|r| *r)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Copy, Clone, Debug, PartialEq)]
    struct TestPoint {
        x: f32,
        y: f32,
    }

    impl Point for TestPoint {
        type Dtype = f32;
        const NUM_DIMENSIONS: u8 = 2;
        fn get_val(&self, dimension: u8) -> Self::Dtype {
            match dimension {
                0 => self.x,
                1 => self.y,
                _ => panic!("Invalid dimension requested"),
            }
        }

        fn dist2(&self, other: &Self) -> f64 {
            ((self.x - other.x).powf(2.0) + (self.y - other.y).powf(2.0)).into()
        }
    }

    #[test]
    fn test_make_kdtree() {
        let points = vec![
            TestPoint { x: 0.0, y: 0.0 },
            TestPoint { x: 0.5, y: -0.5 },
            TestPoint { x: 1.0, y: 0.0 },
            TestPoint { x: 0.0, y: -1.0 },
        ];
        let tree = KDTree::new(points);

        assert_eq!(tree.num_points(), 4);
    }

    #[test]
    fn test_leaf_node() {
        let points = (0..25)
            .map(|i| TestPoint {
                x: (i / 5) as f32,
                y: (i % 5) as f32,
            })
            .collect::<Vec<_>>();
        let tree = KDTree::new(points);

        assert_eq!(
            tree.get_closest(&TestPoint { x: 1.2, y: 1.2 }, 0.0).res,
            Some(TestPoint { x: 1.0, y: 1.0 })
        );

        assert_eq!(
            tree.get_closest(&TestPoint { x: 3.8, y: 1.49 }, 0.0).res,
            Some(TestPoint { x: 4.0, y: 1.0 })
        );
    }

    #[test]
    fn test_multiple_layers() {
        let points = (0..10000)
            .map(|i| TestPoint {
                x: (i / 100) as f32,
                y: (i % 100) as f32,
            })
            .collect::<Vec<_>>();
        let tree = KDTree::new(points);

        assert!(tree.nodes.len() > 10000 / MAX_LEAF_SIZE);

        assert_eq!(
            tree.get_closest(&TestPoint { x: 1.2, y: 1.2 }, 0.0).res,
            Some(TestPoint { x: 1.0, y: 1.0 })
        );

        assert_eq!(
            tree.get_closest(&TestPoint { x: 3.8, y: 1.49 }, 0.0).res,
            Some(TestPoint { x: 4.0, y: 1.0 })
        );
    }

    #[test]
    fn test_valid_indices() {
        let points = (0..10000)
            .map(|i| TestPoint {
                x: (i / 100) as f32,
                y: (i % 100) as f32,
            })
            .collect::<Vec<_>>();
        let tree = KDTree::new(points);

        tree.nodes.iter().for_each(|node| {
            if let Some(parent) = node.parent {
                assert!(parent < tree.nodes.len());
            }

            match node.data {
                NodeData::Internal { left, right, .. } => {
                    assert!(left < tree.nodes.len());
                    assert!(right < tree.nodes.len());
                }
                NodeData::Leaf { i_initial, i_final } => {
                    assert!(i_initial < i_final);
                    assert!(i_initial < tree.points.len());
                    assert!(i_final <= tree.points.len());
                }
            }
        });
    }

    #[test]
    fn test_pop_results() {
        let points = (0..10000)
            .map(|i| TestPoint {
                x: (i / 100) as f32,
                y: (i % 100) as f32,
            })
            .collect::<Vec<_>>();
        let mut tree = KDTree::new(points);

        assert!(tree.nodes.len() > 10000 / MAX_LEAF_SIZE);

        assert_eq!(
            tree.pop_closest(&TestPoint { x: 1.45, y: 1.55 }, 0.0).res,
            Some(TestPoint { x: 1.0, y: 2.0 })
        );

        assert_eq!(
            tree.pop_closest(&TestPoint { x: 1.45, y: 1.55 }, 0.0).res,
            Some(TestPoint { x: 1.0, y: 1.0 })
        );

        assert_eq!(
            tree.pop_closest(&TestPoint { x: 1.45, y: 1.55 }, 0.0).res,
            Some(TestPoint { x: 2.0, y: 2.0 })
        );

        assert_eq!(
            tree.pop_closest(&TestPoint { x: 1.45, y: 1.55 }, 0.0).res,
            Some(TestPoint { x: 2.0, y: 1.0 })
        );

        for _i in 0..9995 {
            assert_ne!(
                tree.pop_closest(&TestPoint { x: 100.0, y: 100.0 }, 0.0).res,
                None
            )
        }

        assert_eq!(
            tree.pop_closest(&TestPoint { x: 100.0, y: 100.0 }, 0.0).res,
            Some(TestPoint { x: 0.0, y: 0.0 })
        );

        assert_eq!(
            tree.pop_closest(&TestPoint { x: 100.0, y: 100.0 }, 0.0).res,
            None
        );
    }

    #[test]
    fn test_epsilon() {
        // This test relies on too many implementation details, maybe
        // should be simplified.  Makes a kd tree that deliberately
        // has two leaf nodes, then look for a point close to the
        // median.  With epsilon==0, the exact point is found, but
        // with more nodes checked.  With epsilon>0, a close point is
        // found, but with fewer leaf nodes checked.
        let q1_points =
            (0..MAX_LEAF_SIZE).map(|_i| TestPoint { x: -1.0, y: -2.0 });
        let q3_points =
            (0..MAX_LEAF_SIZE).map(|_i| TestPoint { x: 1.0, y: 2.0 });
        let points = q1_points.chain(q3_points).collect::<Vec<_>>();

        let tree = KDTree::new(points);
        let res = tree.get_closest(&TestPoint { x: -0.1, y: 2.0 }, 0.0);
        assert_eq!(res.res, Some(TestPoint { x: 1.0, y: 2.0 }));
        assert_eq!(res.stats.leaf_nodes_checked, 2);

        let res = tree.get_closest(&TestPoint { x: -0.1, y: 2.0 }, 5.0);
        assert_eq!(res.res, Some(TestPoint { x: -1.0, y: -2.0 }));
        assert_eq!(res.stats.leaf_nodes_checked, 1);
    }
}
