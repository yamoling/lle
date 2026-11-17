use std::fmt::Display;

use crate::{Position, agent::Colour, tiles::CardinalDirection};

use crate::ParseError;

use super::{
    button_config::ButtonConfig, laser_config::LaserConfig, lever_config::LeverConfig,
    lift_config::LiftConfig, parser_v1::to_v1_string, pressure_plate_config::PressurePlateConfig,
    toml::TomlConfig,
};

/// The static description of a map, as parsed from a v1 string or a TOML file: dimensions,
/// placements, actuators and agent colours.
///
/// It is independent from any world engine. Use [`MapSpec::validate`] to check it and compute the
/// effective start positions, then build a world from the resulting [`ValidMap`].
#[derive(Clone, Debug)]
pub struct MapSpec {
    width: usize,
    height: usize,
    layers: usize,
    gems: Vec<Position>,
    random_starts: Vec<Vec<Position>>,
    voids: Vec<Position>,
    exits: Vec<Position>,
    walls: Vec<Position>,
    /// Initial positions of the movable boxes. Boxes are occupants, not tiles.
    boxes: Vec<Position>,
    lasers: Vec<(Position, LaserConfig)>,
    lifts: Vec<(Position, LiftConfig)>,
    buttons: Vec<(Position, ButtonConfig)>,
    levers: Vec<(Position, LeverConfig)>,
    pressure_plates: Vec<(Position, PressurePlateConfig)>,
    /// The colour of each agent, indexed by agent id. Defaults to the agent id.
    colours: Vec<Colour>,
}

impl MapSpec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        width: usize,
        height: usize,
        layers: usize,
        gem_positions: Vec<Position>,
        random_start_positions: Vec<Vec<Position>>,
        void_positions: Vec<Position>,
        exit_positions: Vec<Position>,
        walls_positions: Vec<Position>,
        box_positions: Vec<Position>,
        source_configs: Vec<(Position, LaserConfig)>,
        lift_configs: Vec<(Position, LiftConfig)>,
        button_configs: Vec<(Position, ButtonConfig)>,
        lever_configs: Vec<(Position, LeverConfig)>,
        pressure_plate_configs: Vec<(Position, PressurePlateConfig)>,
        colours: Vec<Colour>,
    ) -> Self {
        Self {
            width,
            height,
            layers,
            gems: gem_positions,
            random_starts: random_start_positions,
            voids: void_positions,
            exits: exit_positions,
            walls: walls_positions,
            boxes: box_positions,
            lasers: source_configs,
            lifts: lift_configs,
            buttons: button_configs,
            levers: lever_configs,
            pressure_plates: pressure_plate_configs,
            colours,
        }
    }

    pub fn exits(&self) -> &Vec<Position> {
        &self.exits
    }

    pub fn random_starts(&self) -> &Vec<Vec<Position>> {
        &self.random_starts
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn layers(&self) -> usize {
        self.layers
    }

    pub fn voids(&self) -> &Vec<Position> {
        &self.voids
    }

    pub fn walls(&self) -> &Vec<Position> {
        &self.walls
    }

    pub fn boxes(&self) -> &Vec<Position> {
        &self.boxes
    }

    pub fn gems(&self) -> &Vec<Position> {
        &self.gems
    }

    pub fn sources(&self) -> &Vec<(Position, LaserConfig)> {
        &self.lasers
    }

    pub fn lifts(&self) -> &Vec<(Position, LiftConfig)> {
        &self.lifts
    }

    pub fn buttons(&self) -> &Vec<(Position, ButtonConfig)> {
        &self.buttons
    }

    pub fn levers(&self) -> &Vec<(Position, LeverConfig)> {
        &self.levers
    }

    pub fn pressure_plates(&self) -> &Vec<(Position, PressurePlateConfig)> {
        &self.pressure_plates
    }

    /// The colour of each agent, indexed by agent id.
    pub fn colours(&self) -> &Vec<Colour> {
        &self.colours
    }

    pub fn n_agents(&self) -> usize {
        self.random_starts.len()
    }

    /// The cells covered by the beam of a laser source located at `source` and pointing towards
    /// `direction`, in beam order (closest to the source first).
    ///
    /// The beam starts on the neighbour of the source and stops before the first wall, laser
    /// source or the edge of the grid. Nothing else stops a beam: agents, boxes, voids and
    /// actuators only block or kill at runtime. A source on a layer outside of the grid has no
    /// beam.
    ///
    /// @ai-generated
    pub fn beam_cells(&self, source: Position, direction: CardinalDirection) -> Vec<Position> {
        let mut cells = vec![];
        if source.k >= self.layers {
            return cells;
        }
        let (di, dj) = direction.delta();
        let (mut i, mut j) = (source.i as i32 + di, source.j as i32 + dj);
        while i >= 0 && j >= 0 && (i as usize) < self.height && (j as usize) < self.width {
            let pos = Position {
                i: i as usize,
                j: j as usize,
                k: source.k,
            };
            if self.walls.contains(&pos) || self.lasers.iter().any(|(src, _)| *src == pos) {
                break;
            }
            cells.push(pos);
            (i, j) = (i + di, j + dj);
        }
        cells
    }

    /// Validate the map and compute the effective start positions of each agent.
    ///
    /// The parsed description is left untouched: the pruned starts live in the returned
    /// [`ValidMap`]. The checks are, in order:
    /// 1. there is at least one agent and at least as many exits as agents;
    /// 2. no box starts on a wall, void, laser source, agent start or another box;
    /// 3. the random starts are pruned (see below);
    /// 4. every agent keeps at least one start, and there are at least as many starts as agents.
    ///
    /// Pruning removes, for every laser in declaration order, the beam cells from the starts of
    /// the agents of another colour, since they would die there on reset. The beam is blocked
    /// (and the cells behind are kept) from the first cell holding a box, or the only start of an
    /// agent of the laser's colour. Lasers are processed sequentially: an agent left with a single
    /// start by an earlier laser blocks the beams of the later lasers of its colour.
    ///
    /// # Errors
    /// [`ParseError::NoAgents`], [`ParseError::NotEnoughExitTiles`],
    /// [`ParseError::InvalidBoxPosition`], [`ParseError::AgentWithoutStart`] or
    /// [`ParseError::NotEnoughStartTiles`] when the corresponding check fails.
    ///
    /// @ai-generated
    pub fn validate(self) -> Result<ValidMap, ParseError> {
        self.check_agents_exits_and_boxes()?;
        let starts = self.pruned_starts();
        Self::check_starts(&starts)?;
        Ok(ValidMap { spec: self, starts })
    }

    /// The checks that do not depend on the pruned starts.
    ///
    /// @ai-edited
    fn check_agents_exits_and_boxes(&self) -> Result<(), ParseError> {
        // There are some agents
        if self.random_starts.is_empty() {
            return Err(ParseError::NoAgents);
        }
        // There are enough exit tiles
        if self.exits.len() < self.n_agents() {
            return Err(ParseError::NotEnoughExitTiles {
                n_starts: self.n_agents(),
                n_exits: self.exits.len(),
            });
        }

        // Boxes start on a free cell: no wall, void, laser source, agent start or other box.
        for (k, pos) in self.boxes.iter().enumerate() {
            let on_occupied_cell = self.walls.contains(pos)
                || self.voids.contains(pos)
                || self.lasers.iter().any(|(source, _)| source == pos)
                || self
                    .random_starts
                    .iter()
                    .flatten()
                    .any(|start| start == pos)
                || self.boxes[..k].contains(pos);
            if on_occupied_cell {
                return Err(ParseError::InvalidBoxPosition { position: *pos });
            }
        }
        Ok(())
    }

    /// The checks on the effective (pruned) starts.
    ///
    /// @ai-edited
    fn check_starts(starts: &[Vec<Position>]) -> Result<(), ParseError> {
        // All agents have at least one start tile
        for (agent_id, starts) in starts.iter().enumerate() {
            if starts.is_empty() {
                return Err(ParseError::AgentWithoutStart { agent_id });
            }
        }

        // Check that there are enough start tiles for all agents
        let n_starts: usize = starts.iter().map(Vec::len).sum();
        if n_starts < starts.len() {
            return Err(ParseError::NotEnoughStartTiles {
                n_starts,
                n_agents: starts.len(),
            });
        }
        Ok(())
    }

    /// The random starts without the cells under the unblocked beam of another colour.
    /// See [`MapSpec::validate`] for the rule.
    ///
    /// @ai-generated
    fn pruned_starts(&self) -> Vec<Vec<Position>> {
        let mut starts = self.random_starts.clone();
        for (source_pos, laser) in &self.lasers {
            let mut is_blocked = false;
            for pos in self.beam_cells(*source_pos, laser.direction) {
                // An agent of the laser's colour whose only start is on the beam blocks it.
                let same_colour_blocker = starts.iter().enumerate().any(|(agent_id, s)| {
                    self.colours.get(agent_id) == Some(&laser.colour) && s.len() == 1 && s[0] == pos
                });
                // A box standing in the beam blocks it from the very first reset.
                if same_colour_blocker || self.boxes.contains(&pos) {
                    is_blocked = true;
                }
                if is_blocked {
                    break;
                }
                for (agent_id, agent_starts) in starts.iter_mut().enumerate() {
                    if self.colours.get(agent_id) != Some(&laser.colour) {
                        agent_starts.retain(|start| *start != pos);
                    }
                }
            }
        }
        starts
    }
}

/// A [`MapSpec`] that passed [`MapSpec::validate`], along with the effective start positions of
/// each agent (the parsed random starts minus the cells where the agent would die on reset).
#[derive(Debug)]
pub struct ValidMap {
    spec: MapSpec,
    starts: Vec<Vec<Position>>,
}

impl ValidMap {
    /// The validated map description, exactly as parsed.
    pub fn spec(&self) -> &MapSpec {
        &self.spec
    }

    /// The effective start positions of each agent, indexed by agent id.
    pub fn starts(&self) -> &Vec<Vec<Position>> {
        &self.starts
    }

    pub fn n_agents(&self) -> usize {
        self.starts.len()
    }

    /// Give back the map description, dropping the effective starts.
    pub fn into_spec(self) -> MapSpec {
        self.spec
    }
}

impl Display for MapSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(string) = to_v1_string(self) {
            return write!(f, "{string}");
        }
        let toml_config: TomlConfig = self.into();
        write!(f, "{}", toml_config.to_toml_string())
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_map_spec.rs"]
mod tests;
