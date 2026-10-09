use strum::IntoEnumIterator;

use crate::Position;
use crate::solver::{Clause, clauses::ClauseEngine};
use crate::tiles::CardinalDirection;

use super::utils::{at_most_one, implies};

/// Box constraints. `N[x]` denotes `x` and its four adjacent cells.
impl ClauseEngine {
    /// Every box-related clause for step `t`.
    pub(super) fn generate_box_clauses(&mut self, t: usize) -> Vec<Clause> {
        if self.ctx.n_boxes() == 0 {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        clauses.extend(self.box_time_wise_adjacency(t));
        clauses.extend(self.box_inertia(t));
        clauses.extend(self.box_push(t));
        clauses.extend(self.box_at_most_one_position(t));
        clauses.extend(self.box_no_overlap(t));
        clauses.extend(self.box_no_following_conflict(t));
        clauses
    }

    /// A box at `x` at `t` was on a non-void cell of `N[x]` at `t - 1`.
    ///
    /// Refusing voids as predecessors is what destroys a box falling into a void: it has no
    /// possible position afterwards.
    fn box_time_wise_adjacency(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        for box_id in 0..ctx.n_boxes() {
            let previous = ctx.relevant_positions_for_box(box_id, t - 1);
            for x in ctx.relevant_positions_for_box(box_id, t) {
                let mut clause = vec![-pool.box_at(box_id, x, t)];
                for x_prev in closed_neighbourhood(x) {
                    if previous.contains(&x_prev) && !ctx.is_void(&x_prev) {
                        clause.push(pool.box_at(box_id, x_prev, t - 1));
                    }
                }
                clauses.push(clause);
            }
        }
        clauses
    }

    /// A box on a non-void cell stays there unless an agent enters its cell.
    fn box_inertia(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        for box_id in 0..ctx.n_boxes() {
            for q in ctx.relevant_positions_for_box(box_id, t - 1) {
                if ctx.is_void(&q) {
                    continue;
                }
                let mut clause = vec![-pool.box_at(box_id, q, t - 1), pool.box_at(box_id, q, t)];
                for agent in 0..ctx.n_agents {
                    if ctx.relevant_positions_for_agent(agent, t).contains(&q) {
                        clause.push(pool.agent(agent, q, t));
                    }
                }
                clauses.push(clause);
            }
        }
        clauses
    }

    /// An agent stepping from `p = q - d` onto the cell `q` of a box pushes the box to
    /// `r = q + d`, which is impossible if `r` cannot hold a box.
    fn box_push(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        for box_id in 0..ctx.n_boxes() {
            for q in ctx.relevant_positions_for_box(box_id, t - 1) {
                if ctx.is_void(&q) {
                    continue;
                }
                for d in CardinalDirection::iter() {
                    let Some(p) = ctx.pusher_origin(q, d) else {
                        continue;
                    };
                    let destination = ctx.push_destination(q, d);
                    for agent in 0..ctx.n_agents {
                        if !ctx.relevant_positions_for_agent(agent, t - 1).contains(&p)
                            || !ctx.relevant_positions_for_agent(agent, t).contains(&q)
                            || !ctx.neighbours[p.k][p.i][p.j].contains(&q)
                        {
                            continue;
                        }
                        let mut clause = vec![
                            -pool.agent(agent, p, t - 1),
                            -pool.agent(agent, q, t),
                            -pool.box_at(box_id, q, t - 1),
                        ];
                        if let Some(r) = destination {
                            clause.push(pool.box_at(box_id, r, t));
                        }
                        clauses.push(clause);
                    }
                }
            }
        }
        clauses
    }

    /// A box is at most at one position at any time step.
    fn box_at_most_one_position(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for box_id in 0..self.ctx.n_boxes() {
            let vars: Vec<i32> = self
                .ctx
                .relevant_positions_for_box(box_id, t)
                .into_iter()
                .map(|p| self.pool.box_at(box_id, p, t))
                .collect();
            clauses.extend(at_most_one(&vars, &mut self.pool));
        }
        clauses
    }

    /// Two boxes, or an agent and a box, never share a cell.
    ///
    /// Boxes cannot share a void either: `World` refuses two boxes pushed into the same void at once.
    fn box_no_overlap(&mut self, t: usize) -> Vec<Clause> {
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        for b1 in 0..ctx.n_boxes() {
            let positions = ctx.relevant_positions_for_box(b1, t);
            for b2 in b1 + 1..ctx.n_boxes() {
                for pos in positions.intersection(ctx.relevant_positions_for_box(b2, t)) {
                    clauses.push(vec![-pool.box_at(b1, pos, t), -pool.box_at(b2, pos, t)]);
                }
            }
            for agent in 0..ctx.n_agents {
                for pos in positions.intersection(ctx.relevant_positions_for_agent(agent, t)) {
                    clauses.push(vec![-pool.agent(agent, pos, t), -pool.box_at(b1, pos, t)]);
                }
            }
        }
        clauses
    }

    /// A box cannot be pushed onto a cell an agent is leaving.
    ///
    /// The box/box counterpart is implied: a box only leaves its cell when an agent enters it
    /// ([`Self::box_inertia`]), and that agent would then share the cell with the incoming box
    /// ([`Self::box_no_overlap`]).
    fn box_no_following_conflict(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        for b in 0..ctx.n_boxes() {
            let current = ctx.relevant_positions_for_box(b, t);
            for agent in 0..ctx.n_agents {
                let previous = ctx.relevant_positions_for_agent(agent, t - 1);
                for pos in current.intersection(previous) {
                    clauses.push(implies(
                        pool.box_at(b, pos, t),
                        -pool.agent(agent, pos, t - 1),
                    ));
                }
            }
        }
        clauses
    }
}

/// `x` and its four adjacent cells that lie inside the grid's non-negative quadrant.
fn closed_neighbourhood(x: Position) -> impl Iterator<Item = Position> {
    std::iter::once(x).chain(CardinalDirection::iter().filter_map(move |d| (x + d).ok()))
}
