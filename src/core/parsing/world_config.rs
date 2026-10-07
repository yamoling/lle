use std::{collections::HashSet, fmt::Display, vec};

use crate::{
    Grid, Position, World,
    agent::Colour,
    tiles::{Gem, Laser, Tile, Void},
};

use crate::ParseError;

use super::{
    button_config::ButtonConfig, laser_config::LaserConfig, lift_config::LiftConfig,
    parser_v1::to_v1_string, toml::TomlConfig,
};

#[derive(Debug)]
pub struct WorldConfig {
    width: usize,
    height: usize,
    layers: usize,
    gems: Vec<Position>,
    random_starts: Vec<Vec<Position>>,
    voids: Vec<Position>,
    exits: Vec<Position>,
    walls: Vec<Position>,
    lasers: Vec<(Position, LaserConfig)>,
    lifts: Vec<(Position, LiftConfig)>,
    buttons: Vec<(Position, ButtonConfig)>,
    /// The colour of each agent, indexed by agent id. Defaults to the agent id.
    colours: Vec<Colour>,
}

impl WorldConfig {
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
        source_configs: Vec<(Position, LaserConfig)>,
        lift_configs: Vec<(Position, LiftConfig)>,
        button_configs: Vec<(Position, ButtonConfig)>,
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
            lasers: source_configs,
            lifts: lift_configs,
            buttons: button_configs,
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

    /// The colour of each agent, indexed by agent id.
    pub fn colours(&self) -> &Vec<Colour> {
        &self.colours
    }

    /// Whether two agents share a colour, which the solver forbids and the non-perspective
    /// observations refuse to represent.
    pub fn has_shared_colours(&self) -> bool {
        let mut seen = HashSet::new();
        !self.colours.iter().all(|c| seen.insert(c))
    }

    pub fn add_random_starts(&mut self, starts: Vec<Vec<Position>>) {
        for (i, start) in starts.into_iter().enumerate() {
            let start = self.filter_positions(start, &self.walls);
            while i >= self.random_starts.len() {
                self.random_starts.push(vec![]);
            }
            self.random_starts[i].extend(start);
        }
    }

    pub fn add_exits(&mut self, exits: Vec<Position>) {
        let exits = self.filter_positions(exits, &self.walls);
        self.exits.extend(exits);
    }

    pub fn add_gems(&mut self, gems: Vec<Position>) {
        let gems = self.filter_positions(gems, &self.walls);
        self.gems.extend(gems);
    }

    fn filter_positions(&self, positions: Vec<Position>, forbidden: &[Position]) -> Vec<Position> {
        positions
            .into_iter()
            .filter(|pos| !forbidden.contains(pos))
            .collect()
    }

    pub fn into_world(mut self) -> Result<World, ParseError> {
        self.pre_validate()?;
        let (grid, lasers_positions) = self.make_grid();
        self.post_validate()?;
        let source_positions = self.lasers.iter().map(|(pos, _)| *pos).collect();
        let lift_positions = self.lifts.iter().map(|(pos, _)| *pos).collect();
        let button_positions = self.buttons.iter().map(|(pos, _)| *pos).collect();
        Ok(World::new(
            grid,
            self.gems,
            self.random_starts,
            self.voids,
            self.exits,
            self.walls,
            source_positions,
            lasers_positions,
            lift_positions,
            button_positions,
            self.colours,
        ))
    }

    fn pre_validate(&self) -> Result<(), ParseError> {
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

        // // Check that there are no lasers with an agent ID that does not exist
        // for (_, source) in self.lasers.iter() {
        //     if source.agent_id >= self.n_agents() {
        //         return Err(ParseError::InvalidLaserSourceAgentId {
        //             asked_id: source.agent_id,
        //             n_agents: self.n_agents(),
        //         });
        //     }
        // }

        Ok(())
    }

    fn post_validate(&self) -> Result<(), ParseError> {
        // All agents have at least one start tile
        for (agent_id, starts) in self.random_starts.iter().enumerate() {
            if starts.is_empty() {
                return Err(ParseError::AgentWithoutStart { agent_id });
            }
        }

        // Check that there are enough start tiles for all agents
        let n_starts = self
            .random_starts
            .iter()
            .fold(0usize, |sum, current| sum + current.len());
        if n_starts < self.n_agents() {
            return Err(ParseError::NotEnoughStartTiles {
                n_starts,
                n_agents: self.n_agents(),
            });
        }
        Ok(())
    }

    pub fn n_agents(&self) -> usize {
        self.random_starts.len()
    }

    fn make_grid(&mut self) -> (Grid<Tile>, Vec<Position>) {
        let mut grid = Grid::<Tile>::new(self.width, self.height, self.layers).default_init();
        for pos in &self.gems {
            grid.replace_at(pos, Tile::Gem(Gem::default()));
        }
        for pos in &self.exits {
            grid.replace_at(pos, Tile::Exit { agent: None });
        }
        for pos in &self.voids {
            grid.replace_at(pos, Tile::Void(Void::default()));
        }
        for pos in &self.walls {
            grid.replace_at(pos, Tile::Wall);
        }
        for (pos, config) in &self.lifts {
            grid.replace_at(pos, Tile::Lift(config.build()));
        }
        for (pos, config) in &self.buttons {
            grid.replace_at(pos, Tile::Button(config.build()));
        }
        let laser_positions = self.laser_setup(&mut grid).into_iter().collect();
        (grid, laser_positions)
    }

    /// Place the laser sources and wrap the required tiles behind a
    /// `Laser` tile.
    fn laser_setup(&mut self, grid: &mut Grid<Tile>) -> HashSet<Position> {
        let mut laser_positions = HashSet::new();
        let width = grid.width as i32;
        let height: i32 = grid.height as i32;
        for (laser_pos, source) in &self.lasers {
            let mut beam_positions = vec![];
            let delta = source.direction.delta();
            let (mut i, mut j, k) = (laser_pos.i as i32, laser_pos.j as i32, laser_pos.k as i32);
            (i, j) = ((i + delta.0), (j + delta.1));
            if k < 0 || k >= grid.layers as i32 {
                continue; // Invalid layer, skip this laser
            }
            while i >= 0 && j >= 0 && i < height && j < width {
                let pos = Position {
                    i: i as usize,
                    j: j as usize,
                    k: k as usize,
                };
                if !grid.at(&pos).is_walkable() {
                    break;
                }
                beam_positions.push(pos);
                (i, j) = ((i + delta.0), (j + delta.1));
            }
            laser_positions.extend(&beam_positions);
            let source = source.build(beam_positions.len());
            let mut is_blocked = false;
            for (i, pos) in beam_positions.into_iter().enumerate() {
                // Any agent of the source's colour blocks the beam where it spawns.
                for (agent_id, agent_starts) in self.random_starts.iter().enumerate() {
                    if self.colours.get(agent_id) == Some(&source.colour())
                        && agent_starts.len() == 1
                        && agent_starts.contains(&pos)
                    {
                        is_blocked = true;
                    }
                }
                let wrapped = grid.pop(&pos);
                let laser = Tile::Laser(Laser::new(wrapped, source.beam(), i));
                if !is_blocked {
                    // Remove the random starts on this location for agents of another colour,
                    // which would die there on reset.
                    for (start_agent_id, starts) in self.random_starts.iter_mut().enumerate() {
                        if self.colours.get(start_agent_id) == Some(&source.colour()) {
                            continue;
                        }
                        starts.retain(|start| *start != pos);
                    }
                }

                grid.replace_at(&pos, laser);
            }
            grid.replace_at(laser_pos, Tile::LaserSource(source));
        }
        laser_positions
    }
}

impl Display for WorldConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(string) = to_v1_string(self) {
            return write!(f, "{string}");
        }
        let toml_config: TomlConfig = self.into();
        write!(f, "{}", toml_config.to_toml_string())
    }
}
