use std::collections::HashSet;

use crate::{
    tiles::{Gem, Laser, LaserBuilder, LaserSource, Start, Tile, Void},
    AgentId, Position, World,
};

use crate::ParseError;

pub fn parse(world_str: &str) -> Result<World, ParseError> {
    let mut grid = vec![];
    let mut gem_positions: Vec<Position> = vec![];
    let mut start_positions: Vec<(AgentId, Position)> = vec![];
    let mut void_positions: Vec<Position> = vec![];
    let mut exit_positions: Vec<Position> = vec![];
    let mut walls_positions: Vec<Position> = vec![];
    let mut laser_builders: Vec<(Position, LaserBuilder)> = vec![];
    for line in world_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let i = grid.len();
        let mut row = vec![];
        for (j, token) in line.split_whitespace().enumerate() {
            let tile: Tile = match token.to_uppercase().chars().next().unwrap() {
                '.' => Tile::Floor { agent: None },
                '@' => {
                    walls_positions.push((i, j));
                    Tile::Wall
                }
                'G' => {
                    gem_positions.push((i, j));
                    Tile::Gem(Gem::default())
                }
                'S' => {
                    let agent_id = token[1..].parse().map_err(|_| ParseError::InvalidAgentId {
                        given_agent_id: token[1..].into(),
                    })?;
                    // Check for duplicate agent ids
                    for (id, other_pos) in &start_positions {
                        if *id == agent_id {
                            return Err(ParseError::DuplicateStartTile {
                                agent_id,
                                start1: *other_pos,
                                start2: (i, j),
                            });
                        }
                    }
                    start_positions.push((agent_id, (i, j)));
                    Tile::Start(Start::new(agent_id))
                }
                'X' => {
                    exit_positions.push((i, j));
                    Tile::Exit { agent: None }
                }
                'L' => {
                    walls_positions.push((i, j));
                    laser_builders
                        .push(((i, j), LaserSource::from_str(token, laser_builders.len())?));
                    Tile::Wall
                }
                'V' => {
                    void_positions.push((i, j));
                    Tile::Void(Void::default())
                }
                other => {
                    return Err(ParseError::InvalidTile {
                        tile_str: other.into(),
                        line: i,
                        col: j,
                    });
                }
            };
            row.push(tile);
        }
        grid.push(row);
    }
    if grid.is_empty() {
        return Err(ParseError::EmptyWorld);
    }
    // Sort start positions
    start_positions.sort_by(|(id_a, _), (id_b, _)| id_a.cmp(id_b));
    let start_positions: Vec<Position> = start_positions.iter().map(|(_, pos)| *pos).collect();
    // All laser sources have a valid agent id
    let n_agents = start_positions.len();
    let lasers_positions = laser_setup(&mut grid, &mut laser_builders, n_agents)?;
    let source_positions = laser_builders.iter().map(|(pos, _)| *pos).collect();

    // Sanity check
    {
        if start_positions.is_empty() {
            return Err(ParseError::NoAgents);
        }
        // There are enough start/exit tiles
        if start_positions.len() > exit_positions.len() {
            return Err(ParseError::NotEnoughExitTiles {
                n_starts: start_positions.len(),
                n_exits: exit_positions.len(),
            });
        }

        // All rows have the same length
        let width = grid[0].len();
        for (i, row) in grid.iter().enumerate() {
            if row.len() != width {
                return Err(ParseError::InconsistentDimensions {
                    expected_n_cols: width,
                    actual_n_cols: row.len(),
                    row: i,
                });
            }
        }
    }

    Ok(World::new(
        grid,
        gem_positions,
        start_positions,
        void_positions,
        exit_positions,
        walls_positions,
        source_positions,
        lasers_positions.into_iter().collect(),
        world_str,
    ))
}

/// Wrap the tiles behind lasers with a `Laser` tile.
fn laser_setup(
    grid: &mut Vec<Vec<Tile>>,
    laser_builders: &mut [(Position, LaserBuilder)],
    n_agents: usize,
) -> Result<HashSet<Position>, ParseError> {
    let mut laser_positions = HashSet::new();
    let width = grid[0].len() as i32;
    let height: i32 = grid.len() as i32;
    for (pos, source) in laser_builders {
        if source.agent_id >= n_agents {
            return Err(ParseError::InvalidLaserSourceAgentId {
                asked_id: source.agent_id,
                n_agents,
            });
        }
        let delta = source.direction.delta();
        let (mut i, mut j) = (pos.0 as i32, pos.1 as i32);
        (i, j) = ((i + delta.0), (j + delta.1));
        while i >= 0 && j >= 0 && i < height && j < width {
            let pos = (i as usize, j as usize);
            if !grid[pos.0][pos.1].is_waklable() {
                break;
            }
            source.extend_beam(pos);
            (i, j) = ((i + delta.0), (j + delta.1));
        }

        let (source, beam_pos) = source.build();
        for (i, pos) in beam_pos.iter().enumerate() {
            let wrapped = grid[pos.0].remove(pos.1);
            laser_positions.insert(*pos);
            let laser = Tile::Laser(Laser::new(wrapped, source.beam(), i));
            grid[pos.0].insert(pos.1, laser);
        }
        grid[pos.0][pos.1] = Tile::LaserSource(source);
    }
    Ok(laser_positions)
}
