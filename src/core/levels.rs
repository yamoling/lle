pub const LEVELS: [&str; 6] = [
    include_str!("../../resources/levels/lvl1"),
    include_str!("../../resources/levels/lvl2"),
    include_str!("../../resources/levels/lvl3"),
    include_str!("../../resources/levels/lvl4"),
    include_str!("../../resources/levels/lvl5"),
    include_str!("../../resources/levels/lvl6"),
];

pub fn get_level_str(level: &str) -> Option<&'static str> {
    let level = level.to_lowercase();
    let level: usize = if let Some(level) = level.strip_prefix("lvl") {
        level.parse().unwrap()
    } else if let Some(level) = level.strip_prefix("level") {
        level.parse().unwrap()
    } else {
        return None;
    };
    LEVELS.get(level - 1).copied()
}
