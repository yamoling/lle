use crate::{ParseError, Position, World};

use super::parse;

#[test]
fn invalid_toml_field() {
    let toml_content = r#"
    world_string = "S0 X"
    invalid_field = 25
    "#;
    match parse(toml_content) {
        Err(ParseError::UnknownTomlKey { key, .. }) => {
            assert_eq!(key, "invalid_field");
        }
        other => panic!("Should return a ParseError::UnknownTomlKey instead of {other:?}"),
    }
}

#[test]
fn invalid_toml_subfield() {
    let toml_content = r#"
    world_string = "S0 X"
    [[agents]]
    invalid_subfield = 25
    "#;
    match parse(toml_content) {
        Err(ParseError::UnknownTomlKey { key, .. }) => {
            assert_eq!(key, "invalid_subfield");
        }
        other => panic!("Should return a ParseError::UnknownTomlKey instead of {other:?}"),
    }
}

#[test]
fn parse_toml_width_problem() {
    match parse(
        r#"
width = 10
world_string = "S0 X"
"#,
    ) {
        Err(ParseError::InconsistentWorldStringWidth {
            toml_width,
            world_str_width,
        }) => {
            assert_eq!(toml_width, 10);
            assert_eq!(world_str_width, 2);
        }
        _ => panic!("Should return a ParseError::InconsistentWorldStringWidth"),
    }
}

#[test]
fn parse_toml_height_problem() {
    match parse(
        r#"
height = 10
world_string = "S0 X"
"#,
    ) {
        Err(ParseError::InconsistentWorldStringHeight {
            toml_height,
            world_str_height,
        }) => {
            assert_eq!(toml_height, 10);
            assert_eq!(world_str_height, 1);
        }
        _ => panic!("Should return a ParseError::InconsistentWorldStringWidth"),
    }
}

#[test]
fn parse_start_pos_rows() {
    match parse(
        r#"
height = 10
width = 10
n_agents = 2
starts = [{row = 0}]
"#,
    ) {
        Ok(config) => {
            // Start positions should be (0, 0), (0, 1) ... (0, 9)
            for start in config.random_starts() {
                for j in 0..config.width() {
                    assert!(start.contains(&Position { i: 0, j }));
                }
            }
        }
        _ => panic!("The TOML should be parsed successfully"),
    }
}

#[test]
fn parse_start_pos_cols() {
    match parse(
        r#"
height = 10
width = 10
n_agents = 2
starts = [{col = 0}]
"#,
    ) {
        Ok(config) => {
            for start in config.random_starts() {
                for i in 0..config.height() {
                    assert!(start.contains(&Position { i, j: 0 }));
                }
            }
        }
        _ => panic!("The TOML should be parsed successfully"),
    }
}

#[test]
fn parse_start_pos_rows_and_cols() {
    match parse(
        r#"
height = 10
width = 10
n_agents = 2
starts = [{col = 0}, {row=0}]
"#,
    ) {
        Ok(config) => {
            for start in config.random_starts() {
                assert_eq!(
                    start.len(),
                    19,
                    "There should only be 19 starts since duplicates should be removed"
                );
                for i in 0..config.height() {
                    assert!(start.contains(&Position { i, j: 0 }));
                }
                for j in 0..config.width() {
                    assert!(start.contains(&Position { i: 0, j }));
                }
            }
        }
        _ => panic!("The TOML should be parsed successfully"),
    }
}

#[test]
fn start_position_in_wall() {
    let toml_content = r#"
world_string="""
@ . .
@ . X
"""
[[agents]]
start_positions = [{i_min=1}]
    "#;
    let world = World::try_from(toml_content).unwrap();
    let starts = world.possible_starts();
    assert_eq!(1, starts[0].len())
}

#[test]
fn test_ok() {
    let toml_content = r#"
width = 10
height = 5
exits = [{ j_min = 9 }]
gems = [{ i = 0, j = 2 }]
world_string = """
X . . . S1 . . . . .
. . . . .  . . . . .
. . . . .  . . . . .
. . . . .  . . . . .
. . . . .  . . . . .
"""

[[agents]]
start_positions = [{ i_min = 0, i_max = 0 }]

[[agents]]
# Deduced from the string map that agent 1 has a start position at (0, 5).

[[agents]]
start_positions = [{ i = 0, j = 5 }, { i = 3, j = 5 }]

[[agents]]
start_positions = [
    { i = 4, j = 9 },
    { i_min = 1, i_max = 3, j_min = 0, j_max = 3 },
    { j_min = 4 },
]
"#;
    let w = World::try_from(toml_content).unwrap();
    assert_eq!(w.exits_positions().len(), 6);
    assert_eq!(w.gems_positions().len(), 1);
}

#[test]
fn test_global_start_pos() {
    let toml_content = r#"
width = 10
height = 10
n_agents = 5
starts = [{ row = 0 }]
exits = [{ col = 4 }]
"#;
    let w = World::try_from(toml_content).unwrap();
    assert_eq!(w.exits_positions().len(), 10);
    for starts in w.possible_starts() {
        assert_eq!(starts.len(), 9);
    }
}
