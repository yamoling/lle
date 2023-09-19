mod observer;

pub use observer::{Observable, Observer};

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
