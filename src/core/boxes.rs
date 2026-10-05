use crate::Position;

pub type BoxId = usize;

/// The movable boxes of a world. Boxes are occupants of a cell, like agents:
/// the grid itself never changes shape when one moves.
///
/// A destroyed box keeps its last position and is marked absent, exactly as a
/// dead agent keeps its position with `agents_alive[i] = false`. That is what
/// keeps the state vector fixed-size for RL.
pub struct Boxes {
    positions: Vec<Position>,
    present: Vec<bool>,
    initial_positions: Vec<Position>,
    /// Flat `height * width` occupancy index. `None` where no *present* box is.
    occupancy: Vec<Option<BoxId>>,
    width: usize,
    height: usize,
}

impl Boxes {
    pub fn new(initial_positions: Vec<Position>, width: usize, height: usize) -> Self {
        let n = initial_positions.len();
        let mut boxes = Self {
            positions: initial_positions.clone(),
            present: vec![true; n],
            initial_positions,
            occupancy: vec![None; width * height],
            width,
            height,
        };
        boxes.reindex();
        boxes
    }

    fn index(&self, pos: Position) -> usize {
        pos.i * self.width + pos.j
    }

    fn reindex(&mut self) {
        self.occupancy.fill(None);
        for id in 0..self.positions.len() {
            if self.present[id] {
                let index = self.index(self.positions[id]);
                self.occupancy[index] = Some(id);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    #[allow(dead_code)] // used by later tasks (movement, state)
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn positions(&self) -> &Vec<Position> {
        &self.positions
    }

    pub fn present(&self) -> &Vec<bool> {
        &self.present
    }

    pub fn initial_positions(&self) -> &Vec<Position> {
        &self.initial_positions
    }

    /// The present box occupying `pos`, if any.
    pub fn id_at(&self, pos: Position) -> Option<BoxId> {
        if pos.i >= self.height || pos.j >= self.width {
            return None;
        }
        *self.occupancy.get(self.index(pos))?
    }

    #[allow(dead_code)] // used by later tasks (movement, state)
    pub fn set_position(&mut self, id: BoxId, dest: Position) {
        let from = self.index(self.positions[id]);
        self.occupancy[from] = None;
        self.positions[id] = dest;
        if self.present[id] {
            let to = self.index(dest);
            self.occupancy[to] = Some(id);
        }
    }

    pub fn destroy(&mut self, id: BoxId) {
        self.present[id] = false;
        let index = self.index(self.positions[id]);
        self.occupancy[index] = None;
    }

    pub fn reset(&mut self) {
        self.positions.clone_from(&self.initial_positions);
        self.present.clear();
        self.present.resize(self.positions.len(), true);
        self.reindex();
    }

    #[allow(dead_code)] // used by later tasks (movement, state)
    pub fn restore(&mut self, positions: &[Position], present: &[bool]) {
        self.positions.clear();
        self.positions.extend_from_slice(positions);
        self.present.clear();
        self.present.extend_from_slice(present);
        self.reindex();
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_boxes.rs"]
mod test;
