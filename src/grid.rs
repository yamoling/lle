use crate::position::Position;
pub struct Grid<T> {
    pub grid: Vec<T>,
    pub width: usize,
    pub height: usize,
    pub layers: usize,
}

impl<T> Grid<T> {
    /*
     * width is related to the x-axis or j-axis
     * height is related to the y-axis or i-axis
     * layers is related to the z-axis or k-axis
     */
    pub fn new(width: usize, height: usize, layers: usize) -> Self {
        Self {
            grid: Vec::with_capacity(width * height * layers),
            width,
            height,
            layers,
        }
    }

    pub fn replac_at(&mut self, pos: &Position, value: T) {
        if let Some(index) = self.index(pos) {
            self.grid[index] = value;
        } else {
            panic!("Position out of bounds: {:?}", pos);
        }
    }

    pub fn at(&self, pos: &Position) -> &T {
        if let Some(index) = self.index(pos) {
            &self.grid[index]
        } else {
            panic!("Position out of bounds: {:?}", pos);
        }
    }

    pub fn at_mut(&mut self, pos: &Position) -> &mut T {
        if let Some(index) = self.index(pos) {
            &mut self.grid[index]
        } else {
            panic!("Position out of bounds: {:?}", pos);
        }
    }

    pub fn pop(&mut self, pos: &Position) -> T {
        if let Some(index) = self.index(pos) {
            self.grid.remove(index)
        } else {
            panic!("Position out of bounds: {:?}", pos);
        }
    }

    pub fn insert(&mut self, pos: &Position, value: T) {
        if let Some(index) = self.index(pos) {
            self.grid.insert(index, value);
        } else {
            panic!("Position out of bounds: {:?}", pos);
        }
    }

    // Calculate the index in the 1D vector for a given 3D position.
    // can be changed arbitrarily to fit any storage order, but must be consistent as (axis0*dim(axis1*axis2)) + (axis1*dim(axis2)) + axis2
    /// if changed, must also change the `index_to_position` function for iterator construction
    fn index(&self, pos: &Position) -> Option<usize> {
        if pos.i < self.height && pos.j < self.width && pos.k < self.layers {
            Some(pos.k * self.width * self.height + pos.j * self.width + pos.i)
        } else {
            None
        }
    }
    fn index_to_position(&self, index: usize) -> Option<Position> {
        if index < self.grid.len() {
            let k = index / (self.width * self.height);
            let rem = index % (self.width * self.height);
            let j = rem / self.width;
            let i = rem % self.width;
            Some(Position { i, j, k })
        } else {
            None
        }
    }

    pub fn iter(&self) -> GridIterator<T> {
        GridIterator {
            grid: self,
            current: 0,
        }
    }

    pub fn iter_mut(&mut self) -> MutableGridIterator<T> {
        MutableGridIterator {
            grid: self,
            current: 0,
        }
    }
}

pub struct GridIterator<'a, T> {
    grid: &'a Grid<T>,
    current: usize,
}

impl<'a, T> Iterator for GridIterator<'a, T> {
    type Item = (Position, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pos) = self.grid.index_to_position(self.current) {
            let value = &self.grid.grid[self.current];
            self.current += 1;
            Some((pos, value))
        } else {
            None
        }
    }
}

pub struct MutableGridIterator<'a, T> {
    grid: &'a mut Grid<T>,
    current: usize,
}

impl<'a, T> Iterator for MutableGridIterator<'a, T> {
    type Item = (Position, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        //!? need ask for how implement mutable iterator correctly in rust //TODO
        if let Some(pos) = self.grid.index_to_position(self.current) {
            let value = unsafe {
                let ptr = self.grid.grid.as_mut_ptr().add(self.current);
                &mut *ptr
            };
            self.current += 1;
            Some((pos, value))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_index() {
        let grid = Grid::new(10, 10, 3).default_init();
        let pos = Position { i: 2, j: 3, k: 1 };
        assert_eq!(grid.index(&pos), Some(1 * 10 * 10 + 3 * 10 + 2));
        let out_of_bounds_pos = Position { i: 10, j: 3, k: 1 };
        assert_eq!(grid.index(&out_of_bounds_pos), None);
    }

    #[test]
    fn test_grid_index_to_position() {
        let grid = Grid::new(10, 7, 3).default_init();
        let pos = Position { i: 5, j: 3, k: 2 };
        let index = grid.index(&pos).unwrap();
        assert_eq!(grid.index_to_position(index), Some(pos));
    }
}
