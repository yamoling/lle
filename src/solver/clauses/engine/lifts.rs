use std::collections::{HashMap, HashSet};
use std::iter::once;

use strum::IntoEnumIterator;

use crate::Position;
use crate::solver::context::ConstraintContext;
use crate::solver::{
    Clause, Literal, VarKey,
    clauses::{ClauseEngine, VarPool},
};
use crate::tiles::CardinalDirection;

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
        clauses.extend(self.walk_pass_lasers(t));
        clauses
    }

    /// Lasers between the walking part of the step and the lifts: a lifted agent is then still on
    /// its lift, so it neither blocks the beams of its destination nor is safe from the beams of
    /// its lift yet. Only the beams crossing a lift or a lift destination can differ from their
    /// state at `t`, which the laser clauses already constrain.
    fn walk_pass_lasers(&mut self, t: usize) -> Vec<Clause> {
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        // (agent, lift, literal) for every agent that may be lifted in this step.
        let lifted: Vec<(usize, usize, Literal)> = ctx
            .lifts
            .iter()
            .enumerate()
            .flat_map(|(l, lift)| lift.riders.iter().map(move |&agent| (agent, l)))
            .filter_map(|(agent, l)| {
                let key = VarKey::Lifted {
                    agent_id: agent,
                    lift: l,
                    t: t - 1,
                };
                pool.get(&key).map(|lit| (agent, l, lit))
            })
            .collect();
        if lifted.is_empty() {
            return Vec::new();
        }
        let moved: HashSet<Position> = lifted
            .iter()
            .flat_map(|&(_, l, _)| [ctx.lifts[l].pos, ctx.lifts[l].dest])
            .collect();
        let mut clauses = Vec::new();
        let mut occupancy = HashMap::new();
        for source in &ctx.laser_sources {
            if !source.path.iter().any(|pos| moved.contains(pos)) {
                continue;
            }
            // Same chain as `beam_activation`, with the walk-pass occupancy of the owner.
            let mut active = Vec::with_capacity(source.path.len());
            let mut prev_active: Option<Literal> = None;
            for &pos in &source.path {
                let mut blockers: Vec<Literal> = source
                    .owner
                    .and_then(|owner| {
                        walk_occupancy(
                            ctx,
                            pool,
                            &lifted,
                            &moved,
                            &mut occupancy,
                            &mut clauses,
                            owner,
                            pos,
                            t,
                        )
                    })
                    .into_iter()
                    .collect();
                for box_id in 0..ctx.n_boxes() {
                    if !ctx.is_void(&pos)
                        && ctx.relevant_positions_for_box(box_id, t).contains(&pos)
                    {
                        blockers.push(pool.box_at(box_id, pos, t));
                    }
                }
                if !blockers.is_empty() {
                    let lit = pool.aux();
                    let mut activation = Vec::with_capacity(blockers.len() + 2);
                    if let Some(prev) = prev_active {
                        clauses.push(implies(lit, prev));
                        activation.push(-prev);
                    }
                    for &blocker in &blockers {
                        clauses.push(implies(lit, -blocker));
                    }
                    activation.extend(blockers);
                    activation.push(lit);
                    clauses.push(activation);
                    prev_active = Some(lit);
                }
                // `None` before any blocker: a constant-active tile.
                active.push(prev_active);
            }
            for agent in (0..ctx.n_agents).filter(|&agent| Some(agent) != source.owner) {
                for (&pos, &lit) in source.path.iter().zip(&active) {
                    let Some(here) = walk_occupancy(
                        ctx,
                        pool,
                        &lifted,
                        &moved,
                        &mut occupancy,
                        &mut clauses,
                        agent,
                        pos,
                        t,
                    ) else {
                        continue;
                    };
                    clauses.push(match lit {
                        Some(lit) => vec![-here, -lit],
                        None => vec![-here],
                    });
                }
            }
        }
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
                // The rider is on the lift once the walking part of the step is over, even when it
                // walked onto it: no other agent may leave or enter the lift in that step, and no
                // box may be pushed onto it.
                for other in (0..ctx.n_agents).filter(|&other| other != agent) {
                    for step in [t - 1, t] {
                        if ctx
                            .relevant_positions_for_agent(other, step)
                            .contains(&lift.pos)
                        {
                            clauses.push(implies(lifted, -pool.agent(other, lift.pos, step)));
                        }
                    }
                }
                for box_id in 0..ctx.n_boxes() {
                    if ctx
                        .relevant_positions_for_box(box_id, t)
                        .contains(&lift.pos)
                    {
                        clauses.push(implies(lifted, -pool.box_at(box_id, lift.pos, t)));
                    }
                }
                // The world never lifts onto a box: one still on the destination after the walking
                // part of the step was either there before or pushed off by an agent ending there.
                for box_id in 0..ctx.n_boxes() {
                    if ctx
                        .relevant_positions_for_box(box_id, t - 1)
                        .contains(&lift.dest)
                    {
                        clauses.push(implies(lifted, -pool.box_at(box_id, lift.dest, t - 1)));
                    }
                }
                // A rider walking onto a box on the lift pushes it, as in `box_push`.
                for d in CardinalDirection::iter() {
                    let Some(p) = ctx.pusher_origin(lift.pos, d) else {
                        continue;
                    };
                    if !entries.contains(&p) {
                        continue;
                    }
                    let destination = ctx.push_destination(lift.pos, d);
                    for box_id in 0..ctx.n_boxes() {
                        if !ctx
                            .relevant_positions_for_box(box_id, t - 1)
                            .contains(&lift.pos)
                        {
                            continue;
                        }
                        let mut clause = vec![
                            -pool.agent(agent, p, t - 1),
                            -lifted,
                            -pool.box_at(box_id, lift.pos, t - 1),
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

/// The literal of `agent` standing on `pos` once the walking part of the step to `t` is over, or
/// `None` if it cannot. It differs from `agent(agent, pos, t)` only on lifts and their
/// destinations, where an auxiliary variable `w <-> (agent(pos, t) & !lifted_into) | lifted_from`
/// is defined once and cached in `occupancy`.
#[allow(clippy::too_many_arguments)]
fn walk_occupancy(
    ctx: &ConstraintContext,
    pool: &mut VarPool,
    lifted: &[(usize, usize, Literal)],
    moved: &HashSet<Position>,
    occupancy: &mut HashMap<(usize, Position), Option<Literal>>,
    clauses: &mut Vec<Clause>,
    agent: usize,
    pos: Position,
    t: usize,
) -> Option<Literal> {
    let at = ctx
        .relevant_positions_for_agent(agent, t)
        .contains(&pos)
        .then(|| pool.agent(agent, pos, t));
    if !moved.contains(&pos) {
        return at;
    }
    if let Some(&cached) = occupancy.get(&(agent, pos)) {
        return cached;
    }
    let mine = lifted.iter().filter(|&&(a, _, _)| a == agent);
    let into: Vec<Literal> = mine
        .clone()
        .filter(|&&(_, l, _)| ctx.lifts[l].dest == pos)
        .map(|&(_, _, lit)| lit)
        .collect();
    let from: Vec<Literal> = mine
        .filter(|&&(_, l, _)| ctx.lifts[l].pos == pos)
        .map(|&(_, _, lit)| lit)
        .collect();
    let result = if into.is_empty() && from.is_empty() {
        at
    } else {
        let w = pool.aux();
        for &f in &from {
            clauses.push(implies(f, w));
        }
        if let Some(at) = at {
            clauses.push([-at, w].into_iter().chain(into.iter().copied()).collect());
        }
        clauses.push(once(-w).chain(at).chain(from.iter().copied()).collect());
        for &i in &into {
            clauses.push([-w, -i].into_iter().chain(from.iter().copied()).collect());
        }
        Some(w)
    };
    occupancy.insert((agent, pos), result);
    result
}
