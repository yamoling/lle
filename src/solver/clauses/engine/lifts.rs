use std::collections::{HashMap, HashSet};
use std::iter::once;

use strum::IntoEnumIterator;

use crate::Position;
use crate::solver::context::{ConstraintContext, LiftInfo};
use crate::solver::{
    Clause, Literal, VarKey,
    clauses::{ClauseEngine, VarPool},
};
use crate::tiles::CardinalDirection;

use super::utils::implies;

/// Lift clauses, following `World::step`: after the walk, the agents taking `Trigger` press the
/// buttons and toggle the levers they stand on, then every group pulsed by a pressed button, a
/// lever that is on or an occupied pressure plate moves the allowed agent standing on each of its
/// lifts. Step `t` encodes the presses and lifts of the step from `t - 1` to `t`.
impl ClauseEngine {
    /// Every lift-related clause for step `t`. Must run before the clauses that read the
    /// `Lifted` variables through [`Self::lifted_into`].
    pub(super) fn generate_lift_clauses(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let mut clauses = self.triggers(t);
        clauses.extend(self.lever_states(t));
        clauses.extend(self.pulses(t));
        clauses.extend(self.lift_moves(t));
        clauses.extend(self.plate_occupancy(t));
        clauses.extend(self.forced_lifts(t));
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
        let lifted: Vec<_> = lifted_literals(ctx, pool, t, |_| true).collect();
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

    /// A pressed button or a toggled lever has an allowed agent on it, who takes `Trigger` and so
    /// stays there.
    fn triggers(&mut self, t: usize) -> Vec<Clause> {
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        let buttons = ctx.buttons.iter().enumerate().map(|(b, button)| {
            let key = VarKey::Button {
                button: b,
                t: t - 1,
            };
            (key, button)
        });
        let levers = ctx.levers.iter().enumerate().map(|(l, lever)| {
            let key = VarKey::Toggle { lever: l, t: t - 1 };
            (key, lever)
        });
        for (key, actuator) in buttons.chain(levers) {
            let pressers: Vec<usize> = actuator
                .pressers
                .iter()
                .copied()
                .filter(|&agent| {
                    ctx.relevant_positions_for_agent(agent, t - 1)
                        .contains(&actuator.pos)
                        && ctx
                            .relevant_positions_for_agent(agent, t)
                            .contains(&actuator.pos)
                })
                .collect();
            if pressers.is_empty() {
                continue;
            }
            let pressed = pool.id(key);
            let mut someone_on_it = vec![-pressed];
            for agent in pressers {
                let before = pool.agent(agent, actuator.pos, t - 1);
                someone_on_it.push(before);
                clauses.push(vec![-pressed, -before, pool.agent(agent, actuator.pos, t)]);
            }
            clauses.push(someone_on_it);
        }
        clauses
    }

    /// `on(l, t) <-> on(l, t - 1) xor toggle(l, t - 1)`. Levers start off: no `LeverOn` variable
    /// exists until a toggle is possible.
    fn lever_states(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for l in 0..self.ctx.levers.len() {
            let previous = self.pool.get(&VarKey::LeverOn { lever: l, t: t - 1 });
            let toggle = self.pool.get(&VarKey::Toggle { lever: l, t: t - 1 });
            if previous.is_none() && toggle.is_none() {
                continue;
            }
            let on = self.pool.id(VarKey::LeverOn { lever: l, t });
            match (previous, toggle) {
                (Some(p), Some(x)) => clauses.extend([
                    vec![-on, p, x],
                    vec![-on, -p, -x],
                    vec![on, -p, x],
                    vec![on, p, -x],
                ]),
                (Some(same), None) | (None, Some(same)) => {
                    clauses.extend([implies(on, same), implies(same, on)])
                }
                (None, None) => unreachable!(),
            }
        }
        clauses
    }

    /// `pulse(g, t - 1) <-> OR(pressed buttons, levers on at t, occupied plates)` for every group
    /// that has a lift. A lever pulses in the very step it is switched on, and no longer in the
    /// step it is switched off; a plate pulses when an allowed agent is on it after the walk.
    fn pulses(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let mut groups: Vec<usize> = self.ctx.lifts.iter().map(|lift| lift.group).collect();
        groups.sort_unstable();
        groups.dedup();
        for group in groups {
            let mut activators = Vec::new();
            for (b, button) in self.ctx.buttons.iter().enumerate() {
                if button.group == group {
                    activators.extend(self.pool.get(&VarKey::Button {
                        button: b,
                        t: t - 1,
                    }));
                }
            }
            for (l, lever) in self.ctx.levers.iter().enumerate() {
                if lever.group == group {
                    activators.extend(self.pool.get(&VarKey::LeverOn { lever: l, t }));
                }
            }
            for p in 0..self.ctx.plates.len() {
                if self.ctx.plates[p].group == group {
                    for agent in self.ctx.plates[p].pressers.clone() {
                        activators.extend(self.plate_on(p, agent, t));
                    }
                }
            }
            if activators.is_empty() {
                continue;
            }
            let pulse = self.pool.id(VarKey::Pulse { group, t: t - 1 });
            clauses.push(once(-pulse).chain(activators.iter().copied()).collect());
            clauses.extend(
                activators
                    .into_iter()
                    .map(|activator| implies(activator, pulse)),
            );
        }
        clauses
    }

    /// The literal of `agent` standing on plate `p` after the walk of the step to `t`, or `None` if
    /// it cannot. It is `agent(pos, t)`, unless a lift may bring the agent onto the plate: then an
    /// `OnPlate` variable is used, defined by [`Self::plate_occupancy`] once the lifts exist.
    pub(super) fn plate_on(&mut self, p: usize, agent: usize, t: usize) -> Option<Literal> {
        let pos = self.ctx.plates[p].pos;
        if !self
            .ctx
            .relevant_positions_for_agent(agent, t)
            .contains(&pos)
        {
            return None;
        }
        let liftable = self
            .ctx
            .lifts
            .iter()
            .any(|lift| lift.dest == pos && lift.riders.contains(&agent));
        Some(if liftable {
            self.pool.id(VarKey::OnPlate {
                plate: p,
                agent_id: agent,
                t,
            })
        } else {
            self.pool.agent(agent, pos, t)
        })
    }

    /// `on_plate(p, i, t) <-> agent(i, pos, t) & !lifted_into(i, pos, t)`: an agent a lift brings
    /// onto a plate is not on it during the walk.
    fn plate_occupancy(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for (p, plate) in self.ctx.plates.iter().enumerate() {
            for &agent in &plate.pressers {
                let Some(on) = self.pool.get(&VarKey::OnPlate {
                    plate: p,
                    agent_id: agent,
                    t,
                }) else {
                    continue;
                };
                let at = self.pool.agent(agent, plate.pos, t);
                let lifted = self.lifted_into(agent, plate.pos, t);
                clauses.push(implies(on, at));
                for &l in &lifted {
                    clauses.push(implies(on, -l));
                }
                clauses.push([-at, on].into_iter().chain(lifted).collect());
            }
        }
        clauses
    }

    /// A pulsed lift never keeps an allowed agent: it either walked away or was lifted, unless
    /// another lift brought it there in this very step (a lift moves an agent once per step).
    ///
    /// This also covers every case where the world cancels the lift (a box or another agent on
    /// its destination): the solver may not rely on them. Runs after [`Self::lift_moves`], which
    /// creates the `Lifted` variables.
    fn forced_lifts(&mut self, t: usize) -> Vec<Clause> {
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        for lift in &ctx.lifts {
            let Some(pulse) = pool.get(&VarKey::Pulse {
                group: lift.group,
                t: t - 1,
            }) else {
                continue;
            };
            for &agent in &lift.riders {
                if ctx
                    .relevant_positions_for_agent(agent, t)
                    .contains(&lift.pos)
                {
                    let landed: Vec<_> =
                        lifted_literals(ctx, pool, t, |other| other.dest == lift.pos)
                            .filter(|&(a, _, _)| a == agent)
                            .map(|(_, _, lit)| lit)
                            .collect();
                    let stays = pool.agent(agent, lift.pos, t);
                    clauses.push([-pulse, -stays].into_iter().chain(landed).collect());
                }
            }
        }
        clauses
    }

    /// A lifted agent stood on its lift or walked onto it, the lift's group was pulsed, and the
    /// agent lands on the lift's destination.
    fn lift_moves(&mut self, t: usize) -> Vec<Clause> {
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        let mut clauses = Vec::new();
        let mut arrivals = Vec::new();
        for (l, lift) in ctx.lifts.iter().enumerate() {
            let pulses: Vec<Literal> = pool
                .get(&VarKey::Pulse {
                    group: lift.group,
                    t: t - 1,
                })
                .into_iter()
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
                // box may be pushed onto it. Another lift may still bring an agent onto it (two
                // lifts of one group swapping their riders), see `arrivals` below.
                for other in (0..ctx.n_agents).filter(|&other| other != agent) {
                    if ctx
                        .relevant_positions_for_agent(other, t - 1)
                        .contains(&lift.pos)
                    {
                        clauses.push(implies(lifted, -pool.agent(other, lift.pos, t - 1)));
                    }
                    if ctx
                        .relevant_positions_for_agent(other, t)
                        .contains(&lift.pos)
                    {
                        arrivals.push((lifted, other, lift.pos));
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
        // Emitted once every `Lifted` variable of the step exists.
        for (lifted, other, pos) in arrivals {
            let mut clause = vec![-lifted, -pool.agent(other, pos, t)];
            clause.extend(
                lifted_literals(ctx, pool, t, |lift| lift.dest == pos)
                    .filter(|&(a, _, _)| a == other)
                    .map(|(_, _, lit)| lit),
            );
            clauses.push(clause);
        }
        clauses
    }

    /// The `Lifted` literals bringing `agent` onto `pos` during the step from `t - 1` to `t`.
    pub(super) fn lifted_into(&self, agent: usize, pos: Position, t: usize) -> Vec<Literal> {
        lifted_literals(&self.ctx, &self.pool, t, |lift| lift.dest == pos)
            .filter(|&(a, _, _)| a == agent)
            .map(|(_, _, lit)| lit)
            .collect()
    }
}

/// `(agent, lift, literal)` for every existing `Lifted` variable of the step from `t - 1` to `t`
/// whose lift matches `on`. Every clause reading `Lifted` variables must go through this, so that
/// they all agree on which liftings can happen.
pub(super) fn lifted_literals<'a>(
    ctx: &'a ConstraintContext,
    pool: &'a VarPool,
    t: usize,
    on: impl Fn(&LiftInfo) -> bool + 'a,
) -> impl Iterator<Item = (usize, usize, Literal)> + 'a {
    ctx.lifts
        .iter()
        .enumerate()
        .filter(move |(_, lift)| on(lift))
        .flat_map(|(l, lift)| lift.riders.iter().map(move |&agent| (agent, l)))
        .filter_map(move |(agent, l)| {
            let key = VarKey::Lifted {
                agent_id: agent,
                lift: l,
                t: t - 1,
            };
            pool.get(&key).map(|lit| (agent, l, lit))
        })
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
