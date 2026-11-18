//! A static lookup of the cells of the map, so that a tick only looks at the solids and voids
//! near an agent instead of scanning their full lists.
//!
//! The agent radius is below 0.5 (see [`PhysicsParams::validate`](super::PhysicsParams)), so a
//! disc centred in the cell `(i, j)` can only touch the cells of its 3×3 neighbourhood: any
//! other cell is at least one unit away from the centre along one axis. The neighbourhood is
//! taken around the cell that contains the centre (`floor` is exact), rather than around the
//! bounding box of the disc, so that no rounding of `x ± r` can leave a touching cell out. A
//! centre outside the grid takes the neighbourhood of the closest cell of the grid, which still
//! contains every cell that the disc touches.
use crate::shared::Position;

#[derive(Clone, Debug, Default)]
pub(super) struct CellIndex {
    width: usize,
    height: usize,
    /// The solids near each cell (row-major) are
    /// `near_solids[solid_offsets[cell]..solid_offsets[cell + 1]]`.
    solid_offsets: Vec<u32>,
    /// Indices in the list of solids, in increasing order for each cell.
    near_solids: Vec<u32>,
    /// The voids near each cell are `near_voids[void_offsets[cell]..void_offsets[cell + 1]]`.
    void_offsets: Vec<u32>,
    near_voids: Vec<Position>,
    exits: Vec<bool>,
}

impl CellIndex {
    /// Index the cells of `solids`, `voids` and `exits` on a `width`×`height` grid.
    ///
    /// @ai-generated
    pub(super) fn new(
        width: usize,
        height: usize,
        solids: &[Position],
        voids: &[Position],
        exits: &[Position],
    ) -> Self {
        let mut index = Self {
            width,
            height,
            solid_offsets: vec![0],
            near_solids: vec![],
            void_offsets: vec![0],
            near_voids: vec![],
            exits: vec![false; width * height],
        };
        let near = |cell: Position, other: &Position| {
            cell.i.abs_diff(other.i) <= 1 && cell.j.abs_diff(other.j) <= 1
        };
        for i in 0..height {
            for j in 0..width {
                let cell = Position { i, j, k: 0 };
                index
                    .near_solids
                    .extend((0..solids.len() as u32).filter(|&s| near(cell, &solids[s as usize])));
                index.solid_offsets.push(index.near_solids.len() as u32);
                index
                    .near_voids
                    .extend(voids.iter().filter(|v| near(cell, v)).copied());
                index.void_offsets.push(index.near_voids.len() as u32);
            }
        }
        for pos in exits {
            index.exits[pos.i * width + pos.j] = true;
        }
        index
    }

    /// The row-major index of the cell of the grid closest to `p` (the cell that contains it
    /// when it is inside the grid).
    ///
    /// @ai-generated
    fn cell(&self, p: [f64; 2]) -> usize {
        // Saturating float-to-int casts, then clamped to the grid.
        let j = (p[0].floor() as i64).clamp(0, self.width as i64 - 1) as usize;
        let i = (p[1].floor() as i64).clamp(0, self.height as i64 - 1) as usize;
        i * self.width + j
    }

    /// The indices (in the list of solids) of the solids that may touch a disc of radius below
    /// 0.5 centred on `p`, in increasing order, so that a scan of them breaks ties like a scan of
    /// the full list.
    ///
    /// @ai-generated
    pub(super) fn solids_near(&self, p: [f64; 2]) -> &[u32] {
        if self.near_solids.is_empty() {
            return &[];
        }
        let cell = self.cell(p);
        &self.near_solids[self.solid_offsets[cell] as usize..self.solid_offsets[cell + 1] as usize]
    }

    /// The cells of the voids that may touch a disc of radius below 0.5 centred on `p`.
    ///
    /// @ai-generated
    pub(super) fn voids_near(&self, p: [f64; 2]) -> &[Position] {
        if self.near_voids.is_empty() {
            return &[];
        }
        let cell = self.cell(p);
        &self.near_voids[self.void_offsets[cell] as usize..self.void_offsets[cell + 1] as usize]
    }

    /// Whether the point `(x, y)` lies in an exit cell, a cell being closed on its north and
    /// west sides and open on the others.
    ///
    /// @ai-generated
    pub(super) fn in_exit(&self, x: f64, y: f64) -> bool {
        // For x in [j, j + 1), floor(x) is exactly j.
        let (j, i) = (x.floor(), y.floor());
        j >= 0.0
            && i >= 0.0
            && j < self.width as f64
            && i < self.height as f64
            && self.exits[i as usize * self.width + j as usize]
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_continuous_cells.rs"]
mod tests;
