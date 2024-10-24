use std::collections::HashSet;

use rand::{rngs::StdRng, Rng, RngCore};

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

pub fn choose_n_different<T>(seq: &[T], n: usize, rng: &mut impl Rng) -> Vec<T>
where
    T: Clone,
{
    let mut indices = HashSet::with_capacity(n);
    let max_index = seq.len() - 1;
    while indices.len() < n {
        indices.insert(rng.next_u64() as usize % max_index);
    }
    indices.into_iter().map(|i| seq[i].clone()).collect()
}
