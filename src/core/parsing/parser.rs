use std::sync::{Arc, Mutex};

use crate::{
    tiles::{Exit, Floor, Gem, Laser, LaserBuilder, LaserSource, Start, Tile, Void, Wall},
    AgentId, Position, World,
};

use crate::ParseError;

pub fn parse(world_str: &str) -> Result<World, ParseError> {
    let mut grid = vec![];
    let mut gems: Vec<(Position, Arc<Mutex<Gem>>)> = vec![];
    let mut start_positions: Vec<(AgentId, Position)> = vec![];
    let mut void_positions: Vec<Position> = vec![];
    let mut exits: Vec<(Position, Arc<Mutex<Exit>>)> = vec![];
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
            let tile: Arc<Mutex<dyn Tile>> = match token.to_uppercase().chars().next().unwrap() {
                '.' => Arc::<Mutex<Floor>>::default(),
                '@' => {
                    walls_positions.push((i, j));
                    Arc::<Mutex<Wall>>::default()
                }
                'G' => {
                    let gem = Arc::<Mutex<Gem>>::default();
                    gems.push(((i, j), gem.clone()));
                    gem
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
                    Arc::new(Mutex::new(Start::new(agent_id)))
                }
                'X' => {
                    let exit = Arc::<Mutex<Exit>>::default();
                    exits.push(((i, j), exit.clone()));
                    exit
                }
                'L' => {
                    walls_positions.push((i, j));
                    laser_builders
                        .push(((i, j), LaserSource::from_str(token, laser_builders.len())?));
                    Arc::<Mutex<Wall>>::default()
                }
                'V' => {
                    void_positions.push((i, j));
                    Arc::<Mutex<Void>>::default()
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
    let (sources, lasers) = laser_setup(&mut grid, &mut laser_builders);

    // Sanity check
    {
        if start_positions.is_empty() {
            return Err(ParseError::NoAgents);
        }
        // There are enough start/exit tiles
        if start_positions.len() > exits.len() {
            return Err(ParseError::NotEnoughExitTiles {
                n_starts: start_positions.len(),
                n_exits: exits.len(),
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

        // All laser sources have a valid agent id
        let n_agents = start_positions.len();
        for (_, source) in &sources {
            let agent_id = source.lock().unwrap().agent_id();
            if agent_id >= n_agents {
                return Err(ParseError::InvalidLaserSourceAgentId {
                    asked_id: agent_id,
                    n_agents,
                });
            }
        }
    }

    Ok(World::new(
        grid,
        gems,
        lasers,
        sources,
        start_positions,
        void_positions,
        exits,
        walls_positions,
        world_str,
    ))
}

/// Wrap the tiles behind lasers with a `Laser` tile.
fn laser_setup(
    grid: &mut Vec<Vec<Arc<Mutex<dyn Tile>>>>,
    laser_builders: &mut [(Position, LaserBuilder)],
) -> (
    Vec<(Position, Arc<Mutex<LaserSource>>)>,
    Vec<(Position, Arc<Mutex<Laser>>)>,
) {
    let mut lasers = vec![];
    let mut sources = vec![];
    let width = grid[0].len() as i32;
    let height: i32 = grid.len() as i32;
    for (pos, source) in laser_builders {
        let delta = source.direction.delta();
        let (mut i, mut j) = (pos.0 as i32, pos.1 as i32);
        (i, j) = ((i + delta.0), (j + delta.1));
        while i >= 0 && j >= 0 && i < height && j < width {
            let pos = (i as usize, j as usize);
            if !grid[pos.0][pos.1].lock().unwrap().is_waklable() {
                break;
            }
            source.extend_beam(pos);
            (i, j) = ((i + delta.0), (j + delta.1));
        }

        let (source, beam_pos) = source.build();
        for (i, pos) in beam_pos.iter().enumerate() {
            let wrapped = grid[pos.0].remove(pos.1);
            let laser = Arc::new(Mutex::new(Laser::new(wrapped, source.beam(), i)));
            lasers.push((*pos, laser.clone()));
            grid[pos.0].insert(pos.1, laser);
        }
        sources.push((*pos, Arc::new(Mutex::new(source))));
    }
    (sources, lasers)
}
