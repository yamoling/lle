use std::iter::once;

use crate::Position;
use crate::solver::{Clause, Literal, VarKey, clauses::ClauseEngine};

use super::utils::implies;

/// Lift clauses, following `World::step`: an agent pressing a button stays on it, and a pressed
/// button pulses every lift of its group, which moves the allowed agent standing on it once the
/// walking part of the step is over. Step `t` encodes the presses and lifts of the step from
/// `t - 1` to `t`.
impl ClauseEngine {
    /// Every lift-related clause for step `t`. Must run before the clauses that read the
    /// `Lifted` variables through [`Self::lifted_into`].
    pub(super) fn generate_lift_clauses(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let mut clauses = self.button_presses(t);
        clauses.extend(self.forced_lifts(t));
        clauses.extend(self.lift_moves(t));
        clauses
    }

    /// A pressed button has an allowed agent on it, and that agent stays there.
    fn button_presses(&mut self, t: usize) -> Vec<Clause> {
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        for (b, button) in ctx.buttons.iter().enumerate() {
            let pressers: Vec<usize> = button
                .pressers
                .iter()
                .copied()
                .filter(|&agent| {
                    ctx.relevant_positions_for_agent(agent, t - 1)
                        .contains(&button.pos)
                        && ctx
                            .relevant_positions_for_agent(agent, t)
                            .contains(&button.pos)
                })
                .collect();
            if pressers.is_empty() {
                continue;
            }
            let pressed = pool.button(b, t - 1);
            let mut someone_on_it = vec![-pressed];
            for agent in pressers {
                let before = pool.agent(agent, button.pos, t - 1);
                someone_on_it.push(before);
                clauses.push(vec![-pressed, -before, pool.agent(agent, button.pos, t)]);
            }
            clauses.push(someone_on_it);
        }
        clauses
    }

    /// A pulsed lift never keeps an allowed agent: it either walked away or was lifted.
    ///
    /// This also covers every case where the world cancels the lift (a box or another agent on
    /// its destination): the solver may not rely on them.
    fn forced_lifts(&mut self, t: usize) -> Vec<Clause> {
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        for (b, button) in ctx.buttons.iter().enumerate() {
            let Some(pressed) = pool.get(&VarKey::Button {
                button: b,
                t: t - 1,
            }) else {
                continue;
            };
            for lift in ctx.lifts.iter().filter(|lift| lift.group == button.group) {
                for &agent in &lift.riders {
                    if ctx
                        .relevant_positions_for_agent(agent, t)
                        .contains(&lift.pos)
                    {
                        clauses.push(implies(pressed, -pool.agent(agent, lift.pos, t)));
                    }
                }
            }
        }
        clauses
    }

    /// A lifted agent stood on its lift or walked onto it, one of the lift's buttons was pressed,
    /// and the agent lands on the lift's destination.
    fn lift_moves(&mut self, t: usize) -> Vec<Clause> {
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        for (l, lift) in ctx.lifts.iter().enumerate() {
            let pulses: Vec<Literal> = ctx
                .buttons
                .iter()
                .enumerate()
                .filter(|(_, button)| button.group == lift.group)
                .filter_map(|(b, _)| {
                    pool.get(&VarKey::Button {
                        button: b,
                        t: t - 1,
                    })
                })
                .collect();
            if pulses.is_empty() {
                continue;
            }
            for &agent in &lift.riders {
                if !ctx
                    .relevant_positions_for_agent(agent, t)
                    .contains(&lift.dest)
                {
                    continue;
                }
                let previous = ctx.relevant_positions_for_agent(agent, t - 1);
                let entries: Vec<Position> = ctx
                    .lift_entries(lift)
                    .filter(|entry| previous.contains(entry))
                    .collect();
                if entries.is_empty() {
                    continue;
                }
                let lifted = pool.lifted(agent, l, t - 1);
                clauses.push(implies(lifted, pool.agent(agent, lift.dest, t)));
                clauses.push(
                    once(-lifted)
                        .chain(entries.iter().map(|&e| pool.agent(agent, e, t - 1)))
                        .collect(),
                );
                clauses.push(once(-lifted).chain(pulses.iter().copied()).collect());
                // The world never lifts onto a box, and the box clauses would otherwise read the
                // lifted agent as entering (hence pushing) the box on its destination.
                // shortcut: also forbids pushing a box off the lift and riding it in the same
                // step, since `box_push` only models pushers that end the step on the box's cell;
                // model that push if such plans matter.
                for box_id in 0..ctx.n_boxes() {
                    let box_positions = ctx.relevant_positions_for_box(box_id, t - 1);
                    for cell in [lift.dest, lift.pos] {
                        if box_positions.contains(&cell) {
                            clauses.push(implies(lifted, -pool.box_at(box_id, cell, t - 1)));
                        }
                    }
                }
            }
        }
        clauses
    }

    /// The `Lifted` literals bringing `agent` onto `pos` during the step from `t - 1` to `t`.
    pub(super) fn lifted_into(&self, agent: usize, pos: Position, t: usize) -> Vec<Literal> {
        self.ctx
            .lifts
            .iter()
            .enumerate()
            .filter(|(_, lift)| lift.dest == pos)
            .filter_map(|(lift, _)| {
                self.pool.get(&VarKey::Lifted {
                    agent_id: agent,
                    lift,
                    t: t - 1,
                })
            })
            .collect()
    }
}
