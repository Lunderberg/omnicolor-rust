#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct PixelLoc {
    pub i: i32,
    pub j: i32,
}

#[derive(Clone, Copy)]
pub struct RectangularArray {
    pub width: u32,
    pub height: u32,
}

impl RectangularArray {
    pub fn is_valid(&self, loc: PixelLoc) -> bool {
        (loc.i >= 0)
            && (loc.j >= 0)
            && (loc.i < self.width as i32)
            && (loc.j < self.height as i32)
    }

    pub fn get_index(&self, loc: PixelLoc) -> Option<usize> {
        if self.is_valid(loc) {
            Some((loc.j as usize) * (self.width as usize) + (loc.i as usize))
        } else {
            None
        }
    }

    pub fn get_random_loc(&self) -> PixelLoc {
        PixelLoc {
            i: (self.width as f32 * rand::random::<f32>()) as i32,
            j: (self.height as f32 * rand::random::<f32>()) as i32,
        }
    }

    fn _get_loc(&self, index: usize) -> Option<PixelLoc> {
        if index < self.len() {
            Some(PixelLoc {
                i: (index % (self.width as usize)) as i32,
                j: (index / (self.width as usize)) as i32,
            })
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }
}
