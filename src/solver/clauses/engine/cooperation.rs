use std::iter::once;

use itertools::Itertools;

use super::utils::implies;
use crate::solver::{
    Clause, Literal, VarKey,
    clauses::{ClauseEngine, VarPool},
};

impl ClauseEngine {
    /// Generate `Help(helper, beneficiary, t)` variables for time step `t`.
    ///
    /// A help event `helper → beneficiary` at `t` is either:
    /// - **laser help:** the beneficiary stands on one of the helper's laser beam tiles with no box
    ///   upstream on that beam, an occupancy that is only survivable when the helper blocks the beam
    ///   (enforced by the laser clauses, not here). A beam blocked by a box is never help;
    /// - **lift help:** during the step from `t - 1` to `t`, the helper pressed a button that lifted
    ///   the beneficiary.
    ///
    /// ```text
    /// help(helper, beneficiary, t) <=> OR_events event
    /// ```
    ///
    /// Beam positions are pooled across **all** of the helper's laser sources, so a single `Help`
    /// variable covers every beam the helper owns. A `Help` variable is materialized **only** when
    /// at least one event is geometrically possible at `t`; otherwise no variable or clause is
    /// emitted.
    pub fn generate_help_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = vec![];
        // "Some box stands upstream of this beam tile", per source and beam tile.
        let box_upstream: Vec<Vec<Option<Literal>>> = (0..self.ctx.laser_sources.len())
            .map(|s| self.box_upstream(s, t, &mut clauses))
            .collect();
        for agents in (0..self.ctx.n_agents).permutations(2) {
            let helper = agents[0];
            let beneficiary = agents[1];
            let mut events = vec![];
            for (s, source) in self.ctx.laser_sources.iter().enumerate() {
                if source.owner != Some(helper) {
                    continue;
                }
                let relevant = self.ctx.relevant_laser_tiles(source.laser_id, t);
                for (&pos, &boxed) in source.path.iter().zip(&box_upstream[s]) {
                    if !relevant.contains(&pos) {
                        continue;
                    }
                    // Only reachable occupancies, i.e. the movement layer materialized the variable.
                    let Some(on_beam) = self.pool.get(&VarKey::Agent {
                        agent_id: beneficiary,
                        pos,
                        t,
                    }) else {
                        continue;
                    };
                    events.push(match boxed {
                        None => on_beam,
                        Some(boxed) => and(&mut self.pool, &[on_beam, -boxed], &mut clauses),
                    });
                }
            }
            if t > 0 {
                for (l, lift) in self.ctx.lifts.iter().enumerate() {
                    if !lift.riders.contains(&beneficiary) {
                        continue;
                    }
                    let Some(lifted) = self.pool.get(&VarKey::Lifted {
                        agent_id: beneficiary,
                        lift: l,
                        t: t - 1,
                    }) else {
                        continue;
                    };
                    for (b, button) in self.ctx.buttons.iter().enumerate() {
                        if button.group != lift.group || !button.pressers.contains(&helper) {
                            continue;
                        }
                        let pressed = self.pool.get(&VarKey::Button {
                            button: b,
                            t: t - 1,
                        });
                        let on_button = self.pool.get(&VarKey::Agent {
                            agent_id: helper,
                            pos: button.pos,
                            t: t - 1,
                        });
                        // A pressed button holds a single agent, so the agent on it is the presser.
                        if let (Some(pressed), Some(on_button)) = (pressed, on_button) {
                            events.push(and(
                                &mut self.pool,
                                &[lifted, pressed, on_button],
                                &mut clauses,
                            ));
                        }
                    }
                }
            }
            // No possible event: the help event is impossible, so create nothing
            if events.is_empty() {
                continue;
            }
            // Crossing beams can share a tile; keep every literal at most once.
            events.sort_unstable();
            events.dedup();
            let help = self.pool.help(helper, beneficiary, t);
            // event -> help
            for &event in &events {
                clauses.push(implies(event, help));
            }
            // help -> OR(events)
            let mut clause = Vec::with_capacity(1 + events.len());
            clause.push(-help);
            clause.extend(events);
            clauses.push(clause);
        }
        clauses
    }

    /// For every tile of source `s`'s path, a literal for "a box stands on an earlier tile of the
    /// beam at `t`", or `None` when no box can. A box on a void is destroyed and blocks nothing.
    fn box_upstream(
        &mut self,
        s: usize,
        t: usize,
        clauses: &mut Vec<Clause>,
    ) -> Vec<Option<Literal>> {
        let ctx = &self.ctx;
        let mut upstream = None;
        let mut result = Vec::with_capacity(ctx.laser_sources[s].path.len());
        for &pos in &ctx.laser_sources[s].path {
            result.push(upstream);
            let boxes: Vec<Literal> = (0..ctx.n_boxes())
                .filter(|&box_id| {
                    !ctx.is_void(&pos) && ctx.relevant_positions_for_box(box_id, t).contains(&pos)
                })
                .map(|box_id| self.pool.box_at(box_id, pos, t))
                .collect();
            if boxes.is_empty() {
                continue;
            }
            // next <-> upstream | box_1 | ... | box_n
            let next = self.pool.aux();
            clauses.push(
                once(-next)
                    .chain(upstream)
                    .chain(boxes.iter().copied())
                    .collect(),
            );
            for lit in upstream.into_iter().chain(boxes) {
                clauses.push(implies(lit, next));
            }
            upstream = Some(next);
        }
        result
    }

    /// Return assumptions asserting no cooperation at exactly time `t`: no `Help` variable of `t`
    /// holds. Must run after [`Self::generate_help_clauses`] for `t`.
    pub(crate) fn assume_no_cooperation_at(&mut self, t: usize) -> Vec<Literal> {
        (0..self.ctx.n_agents)
            .permutations(2)
            .filter_map(|agents| {
                self.pool.get(&VarKey::Help {
                    helper: agents[0],
                    beneficiary: agents[1],
                    t,
                })
            })
            .map(|help| -help)
            .collect()
    }

    /// Encode `is_helped(beneficiary) <=> OR(help_variables)` over the prefix `0..=horizon`.
    ///
    /// This function must be called for each horizon.
    pub fn generate_is_helped(&mut self, horizon: usize) -> Vec<Clause> {
        let mut clauses = vec![];
        for beneficiary in 0..self.ctx.n_agents {
            // Only create the "is_helped" variable if the agent can actually be helped.
            let beneficiary_variables = self.pool.beneficiary_variables(beneficiary, horizon);
            if beneficiary_variables.is_empty() {
                continue;
            }
            let is_helped = self.pool.is_helped(beneficiary, horizon);
            // is_helped <=> OR_{beneficiary variables}
            // 1) Forward: is_helped -> OR_{beneficiary variables}
            let mut forward = Vec::with_capacity(1 + beneficiary_variables.len());
            forward.push(-is_helped);
            forward.extend(beneficiary_variables.iter());
            clauses.push(forward);
            // 2) Backward: b -> is_helped (for all b in beneficiary variables)
            for v in beneficiary_variables {
                clauses.push(implies(v, is_helped));
            }
        }
        clauses
    }

    /// Encode `provides_help(helper) <=> OR(helper_variables)` over the prefix `0..=horizon`.
    ///
    /// This function should be called for each time horizon.
    pub fn generate_provides_help(&mut self, horizon: usize) -> Vec<Clause> {
        let mut clauses = vec![];
        for helper in 0..self.ctx.n_agents {
            let helper_variables = self.pool.helper_variables(helper, horizon);
            if helper_variables.is_empty() {
                continue;
            }
            let provides_help = self.pool.provides_help_up_to(helper, horizon);
            // provides_help <=> OR(helper_variables)
            // 1) Forward: provides_help => OR(helper_variables)
            let mut forward = Vec::with_capacity(1 + helper_variables.len());
            forward.push(-provides_help);
            forward.extend(helper_variables.iter());
            clauses.push(forward);
            // 2) Backward: h -> provides_help (for all h in helper variables)
            for v in helper_variables {
                clauses.push(implies(v, provides_help));
            }
        }
        clauses
    }

    /// Encode the global `asymmetric` flag over the prefix `0..=horizon`.
    ///
    /// Asymmetry is defined as: there exists an agent that provides help but is never helped, i.e.
    /// ```text
    /// asymmetric <=> OR_{i in A} (provides_help(i) ∧ ¬is_helped(i))
    /// ```
    pub fn encode_asymmetry(&mut self, horizon: usize) -> Vec<Clause> {
        let asymmetric = self.pool.asymmetric(horizon);
        let mut clauses = Vec::new();
        // Collects one literal per agent-level disjunct of the outer OR.
        let mut disjuncts = Vec::with_capacity(self.ctx.n_agents);
        for i in 0..self.ctx.n_agents {
            // Skip agents that can never provide help
            if !self
                .pool
                .exists(&VarKey::ProvidesHelp { helper: i, horizon })
            {
                continue;
            }
            let provides_help = self
                .pool
                .get(&VarKey::ProvidesHelp { helper: i, horizon })
                .expect("Help variables not initialized");
            let beneficiary_variables = self.pool.beneficiary_variables(i, horizon);
            // Agents that can never be helped have no `is_helped` variable,
            // therefore `¬is_helped` is always true and the conjunction reduces to `provides_help`.
            if beneficiary_variables.is_empty() {
                clauses.push(implies(provides_help, asymmetric));
                disjuncts.push(provides_help);
                continue;
            }
            // A conjunction cannot appear bare inside a CNF clause, so each disjunct
            // `provides_help(i) ∧ ¬is_helped(i)` is reified into an auxiliary variable `a_i`.
            let is_helped = self
                .pool
                .get(&VarKey::IsHelped {
                    beneficiary: i,
                    horizon,
                })
                .expect("is_helped variable not initialized");
            // Reify a_i <=> provides_help(i) ∧ ¬is_helped(i).
            let a = self.pool.aux();
            clauses.push(implies(a, provides_help)); // a -> provides_help
            clauses.push(implies(a, -is_helped)); // a -> ¬is_helped
            clauses.push(vec![-provides_help, is_helped, a]); // provides_help ∧ ¬is_helped -> a
            // Backward: a -> asymmetric.
            clauses.push(implies(a, asymmetric));
            disjuncts.push(a);
        }
        // Forward: asymmetric -> OR a_i. With no disjuncts this is the unit clause `¬asymmetric`,
        // pinning the flag to false since no agent can ever be asymmetrically helped.
        let mut forward = Vec::with_capacity(1 + disjuncts.len());
        forward.push(-asymmetric);
        forward.extend(disjuncts);
        clauses.push(forward);
        clauses
    }

    /// Return the assumption that forbids asymmetric help events.
    pub fn assume_no_asymmetry(&mut self, horizon: usize) -> Vec<Literal> {
        vec![-self.pool.asymmetric(horizon)]
    }
}

/// A fresh literal defined as the conjunction of `lits`.
fn and(pool: &mut VarPool, lits: &[Literal], clauses: &mut Vec<Clause>) -> Literal {
    let and = pool.aux();
    for &lit in lits {
        clauses.push(implies(and, lit));
    }
    clauses.push(once(and).chain(lits.iter().map(|&lit| -lit)).collect());
    and
}

#[cfg(test)]
#[path = "../../../unit_tests/engine/test_help.rs"]
mod tests;
