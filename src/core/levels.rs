pub const LEVELS: [&str; 6] = [
    include_str!("../../resources/levels/lvl1"),
    include_str!("../../resources/levels/lvl2"),
    include_str!("../../resources/levels/lvl3"),
    include_str!("../../resources/levels/lvl4"),
    include_str!("../../resources/levels/lvl5"),
    include_str!("../../resources/levels/lvl6"),
];

/// The content of the built-in level named `lvl<n>` or `level<n>` (case-insensitive), or `None`
/// if `level` does not name an existing built-in level.
///
/// @ai-edited
pub fn get_level_str(level: &str) -> Option<&'static str> {
    let level = level.to_lowercase();
    let number = level
        .strip_prefix("lvl")
        .or_else(|| level.strip_prefix("level"))?;
    let index = number.parse::<usize>().ok()?.checked_sub(1)?;
    LEVELS.get(index).copied()
}
