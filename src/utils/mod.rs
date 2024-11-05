use rand::{seq::SliceRandom, Rng};

use crate::Position;

/// Find duplicates in a slice of values.
/// Returns a vector of booleans where the value at index i is true if the value at index i is a duplicate.
pub fn find_duplicates<T>(input: &[T]) -> Vec<bool>
where
    T: PartialEq,
{
    let mut result = vec![false; input.len()]; // Initialize the result vector with false values

    for i in 0..input.len() {
        if !result[i] {
            // Skip if the current value is already marked as a duplicate
            for j in (i + 1)..input.len() {
                if input[i] == input[j] {
                    result[i] = true;
                    result[j] = true;
                }
            }
        }
    }

    result
}

/// Get a random position for each agent such that no two agents start at the same position.
pub fn sample_different(
    rng: &mut impl Rng,
    random_start_positions: &Vec<Vec<Position>>,
) -> Vec<Position> {
    let mut result = Vec::with_capacity(random_start_positions.len());
    // Sort agents by the number of possible positions
    let mut agent_indices: Vec<usize> = (0..random_start_positions.len()).collect();
    agent_indices.sort_by_key(|&i| random_start_positions[i].len());

    /// Recursive backtracking function to assign positions.
    /// Considers the agent with the least possible positions first,
    /// and tries to assign a position to it. If the position is already taken,
    /// it tries the next possible position. If no possible position is available,
    /// it backtracks to the previous agent and tries the next possible position.
    fn assign_positions(
        i: usize,
        agent_indices: &Vec<usize>,
        random_start_positions: &Vec<Vec<Position>>,
        rng: &mut impl Rng,
        result: &mut Vec<Position>,
    ) -> bool {
        let agent_id = agent_indices[i];
        let mut possible_positions = random_start_positions[agent_id].clone();
        if possible_positions.len() > 1 {
            possible_positions.shuffle(rng);
        }

        for pos in possible_positions {
            if !result.contains(&pos) {
                result.push(pos);
                if i + 1 < agent_indices.len() {
                    assign_positions(i + 1, agent_indices, random_start_positions, rng, result);
                }
                if result.len() == agent_indices.len() {
                    return true;
                }
            }
        }
        return false;
    }
    if assign_positions(0, &agent_indices, &random_start_positions, rng, &mut result) {
        result
    } else {
        panic!("Could not assign positions to agents");
    }
}

#[cfg(test)]
mod test;
