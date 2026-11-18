//! A static lookup of the cells of the map, so that a tick only looks at the cells near an
//! agent instead of scanning the lists of solids, voids and exits.
//!
//! The agent radius is below 0.5 (see [`PhysicsParams::validate`](super::PhysicsParams)), so a
//! disc centred in the cell `(i, j)` can only touch the cells of its 3×3 neighbourhood: any
//! other cell is at least one unit away from the centre along one axis. The neighbourhood is
//! taken around the cell that contains the centre (`floor` is exact), rather than around the
//! bounding box of the disc, so that no rounding of `x ± r` can leave a touching cell out.
use crate::shared::Position;

/// No solid on this cell.
const NO_SOLID: u32 = u32::MAX;

#[derive(Clone, Debug, Default)]
pub(super) struct CellIndex {
    width: usize,
    height: usize,
    /// The index in the list of solids of the first solid on each cell (row-major), or
    /// [`NO_SOLID`].
    solids: Vec<u32>,
    voids: Vec<bool>,
    exits: Vec<bool>,
}

/// The indices of up to 9 cells or solids, without allocation.
#[derive(Clone, Copy, Debug)]
pub(super) struct Near {
    items: [u32; 9],
    len: usize,
}

impl Near {
    pub(super) fn as_slice(&self) -> &[u32] {
        &self.items[..self.len]
    }
}

impl CellIndex {
    /// Index the cells of `solids`, `voids` and `exits` on a `width`×`height` grid. A cell that
    /// appears more than once in `solids` keeps its first index.
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
            solids: vec![NO_SOLID; width * height],
            voids: vec![false; width * height],
            exits: vec![false; width * height],
        };
        for (s, pos) in solids.iter().enumerate() {
            let cell = &mut index.solids[pos.i * width + pos.j];
            if *cell == NO_SOLID {
                *cell = s as u32;
            }
        }
        for pos in voids {
            index.voids[pos.i * width + pos.j] = true;
        }
        for pos in exits {
            index.exits[pos.i * width + pos.j] = true;
        }
        index
    }

    /// The row-major indices of the cells of the 3×3 neighbourhood of the cell that contains
    /// `p`, clipped to the grid, in increasing order. Every cell that a disc of radius below 0.5
    /// centred on `p` touches is among them.
    ///
    /// @ai-generated
    fn neighbourhood(&self, p: [f64; 2]) -> Near {
        let mut near = Near {
            items: [0; 9],
            len: 0,
        };
        // Saturating float-to-int casts: a far-away point gives an empty neighbourhood.
        let (cj, ci) = (p[0].floor() as i64, p[1].floor() as i64);
        for i in (ci - 1).max(0)..=(ci + 1).min(self.height as i64 - 1) {
            for j in (cj - 1).max(0)..=(cj + 1).min(self.width as i64 - 1) {
                near.items[near.len] = (i as usize * self.width + j as usize) as u32;
                near.len += 1;
            }
        }
        near
    }

    /// The indices (in the list of solids) of the solids that may touch a disc of radius below
    /// 0.5 centred on `p`, in increasing order, so that a scan of them breaks ties like a scan of
    /// the full list.
    ///
    /// @ai-generated
    pub(super) fn solids_near(&self, p: [f64; 2]) -> Near {
        let mut near = Near {
            items: [0; 9],
            len: 0,
        };
        for &cell in self.neighbourhood(p).as_slice() {
            let s = self.solids[cell as usize];
            if s != NO_SOLID {
                // Insertion sort: there are at most 9 items.
                let mut k = near.len;
                while k > 0 && near.items[k - 1] > s {
                    near.items[k] = near.items[k - 1];
                    k -= 1;
                }
                near.items[k] = s;
                near.len += 1;
            }
        }
        near
    }

    /// The cells of the voids that may touch a disc of radius below 0.5 centred on `p`.
    ///
    /// @ai-generated
    pub(super) fn voids_near(&self, p: [f64; 2]) -> impl Iterator<Item = Position> + '_ {
        let near = self.neighbourhood(p);
        (0..near.len)
            .map(move |k| near.items[k] as usize)
            .filter(|&cell| self.voids[cell])
            .map(|cell| Position {
                i: cell / self.width,
                j: cell % self.width,
                k: 0,
            })
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
