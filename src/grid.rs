use crate::position::Position;
use std::{fmt, string::String};
pub struct Grid<T> {
    pub grid: Vec<Option<T>>,
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

    pub fn replace_at(&mut self, pos: &Position, value: T) {
        match self.index(pos) {
            Some(index) => self.grid[index] = Some(value),
            Option::None => panic!("Position out of bounds: {:?}", pos),
        }
    }

    pub fn at(&self, pos: &Position) -> &T {
        // double matching first for outer bound chekcing and second inner for type exist
        match self.index(pos) {
            Some(index) => match self.grid[index] {
                Some(ref value) => value,
                Option::None => panic!("Position is empty: {:?}", pos),
            },
            Option::None => panic!("Position out of bounds: {:?}", pos),
        }
    }

    pub fn at_mut(&mut self, pos: &Position) -> &mut T {
        match self.index(pos) {
            Some(index) => match self.grid[index] {
                Some(ref mut value) => value,
                Option::None => panic!("Position is empty: {:?}", pos),
            },
            Option::None => panic!("Position out of bounds: {:?}", pos),
        }
    }

    pub fn pop(&mut self, pos: &Position) -> T {
        match self.index(pos) {
            Some(index) => match self.grid[index].take() {
                Some(value) => value,
                Option::None => panic!("Position is empty: {:?}", pos),
            },
            Option::None => panic!("Position out of bounds: {:?}", pos),
        }
    }

    pub fn insert(&mut self, pos: &Position, value: T) {
        match self.index(pos) {
            Some(index) => match self.grid[index] {
                Some(_) => panic!(
                    "Position is already occupied: {:?}, use the function replace_at()",
                    pos
                ),
                Option::None => self.grid[index] = Some(value),
            },
            Option::None => panic!("Position out of bounds: {:?}", pos),
        }
    }

    pub fn is_empty(&self, pos: &Position) -> bool {
        match self.index(pos) {
            Some(index) => self.grid[index].is_none(),
            Option::None => panic!("Position out of bounds: {:?}", pos),
        }
    }
    // Calculate the index in the 1D vector for a given 3D position.
    // can be changed arbitrarily to fit any storage order, but must be consistent as (axis0*dim(axis1*axis2)) + (axis1*dim(axis2)) + axis2
    /// if changed, must also change the `index_to_position` function for iterator construction
    /// reworked: this does not check of existance maybe later do that chekc
    fn index(&self, pos: &Position) -> Option<usize> {
        if pos.i < self.height && pos.j < self.width && pos.k < self.layers {
            Some(pos.k * self.width * self.height + pos.j * self.height + pos.i)
        } else {
            None
        }
    }

    pub fn iter(&'_ self) -> GridIterator<'_, T> {
        GridIterator {
            grid: self.grid.iter(),
            current: 0,
            width: self.width,
            height: self.height,
            layers: self.layers,
        }
    }

    pub fn into_iter(&'_ self) -> GridIterator<'_, T> {
        GridIterator {
            grid: self.grid.iter(),
            current: 0,
            width: self.width,
            height: self.height,
            layers: self.layers,
        }
    }

    pub fn iter_mut(&'_ mut self) -> MutableGridIterator<'_, T> {
        MutableGridIterator {
            grid: self.grid.iter_mut(),
            current: 0,
            width: self.width,
            height: self.height,
            layers: self.layers,
        }
    }
}

/*
important note here as for the iterator implementation there were 2 choice available in my case if the iterator get a empty cell (None) it will be either
- stopped with an error code this or skip the current loop both using a simple Type T without option managment later on (used this one for now)
- continue by sending whatever the cell contained but need to manage the Option downstream
*/
pub struct GridIterator<'a, T> {
    grid: std::slice::Iter<'a, Option<T>>,
    current: usize,
    width: usize,
    height: usize,
    layers: usize,
}

impl<'a, T> Iterator for GridIterator<'a, T> {
    type Item = (Position, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(cell) = self.grid.next() {
            let idx = self.current;
            self.current += 1;
            if let Some(value) = cell.as_ref() {
                if let Some(pos) =
                    index_grid_position_convertor(idx, self.width, self.height, self.layers)
                {
                    return Some((pos, value));
                }
                panic!(
                    "Failed to convert index {} to position given that the cell exists",
                    idx
                );
            }
        }
        None
    }
}

pub struct MutableGridIterator<'a, T> {
    grid: std::slice::IterMut<'a, Option<T>>,
    current: usize,
    width: usize,
    height: usize,
    layers: usize,
}

impl<'a, T> Iterator for MutableGridIterator<'a, T> {
    type Item = (Position, &'a mut T);

    //get a mutable reference item of the Some(element) (skip in case of None)
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(cell) = self.grid.next() {
            let idx = self.current;
            self.current += 1;
            if let Some(value) = cell.as_mut() {
                if let Some(pos) =
                    index_grid_position_convertor(idx, self.width, self.height, self.layers)
                {
                    return Some((pos, value));
                }
                panic!(
                    "Failed to convert index {} to position given that the cell exists",
                    idx
                );
            }
        }
        None
    }
}

impl Grid<String> {
    pub fn default_init(self) -> Self {
        let size = self.width * self.height * self.layers;
        let mut grid = self.grid;
        for _ in 0..size {
            grid.push(Some(String::from(".")));
        }
        Self { grid, ..self }
    }
}

impl Into<String> for &Grid<String> {
    // note that for loop are here due to the vector not totaly consistent and order are not correctly done if using .iter or .fold methode
    fn into(self) -> String {
        let mut output = String::new();
        for k in 0..self.layers {
            for i in 0..self.height {
                for j in 0..self.width {
                    let pos = Position { i, j, k };
                    output.push_str(&format!("{} ", self.at(&pos)));
                }
                output.push('\n');
            }
            if k < self.layers - 1 {
                output.push_str(";\n");
            }
        }
        output
    }
}

impl fmt::Display for Grid<String> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let output: String = self.into();
        writeln!(f, "{}", output)
    }
}

fn index_grid_position_convertor(
    index: usize,
    width: usize,
    height: usize,
    layers: usize,
) -> Option<Position> {
    let total_cells = width * height * layers;
    if index >= total_cells {
        return None;
    }
    let k = index / (width * height);
    let remaining = index % (width * height);
    let j = remaining / width;
    let i = remaining % width;
    Some(Position { i, j, k })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_index() {
        let grid = Grid::<String>::new(10, 10, 3).default_init();
        let pos = Position { i: 2, j: 3, k: 1 };
        assert_eq!(grid.index(&pos), Some(1 * 10 * 10 + 3 * 10 + 2));
        let out_of_bounds_pos = Position { i: 10, j: 3, k: 1 };
        assert_eq!(grid.index(&out_of_bounds_pos), None);
    }

    #[test]
    fn test_grid_index_out_of_bounds() {
        let grid = Grid::<String>::new(10, 10, 3).default_init();
        let pos = Position { i: 10, j: 10, k: 3 };
        assert_eq!(grid.index(&pos), None);
    }

    #[test]
    fn test_iterator_correct_position() {
        let grid = Grid::<String>::new(7, 10, 3).default_init();
        let mut positions = vec![];
        for (pos, _) in grid.iter() {
            positions.push(pos);
        }
        assert_eq!(positions.len(), 7 * 10 * 3);
        let expected_positions: Vec<Position> = (0..3)
            .flat_map(|k| (0..10).flat_map(move |j| (0..7).map(move |i| Position { i, j, k })))
            .collect();
        assert_eq!(positions, expected_positions);
    }
}
