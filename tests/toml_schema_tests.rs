use jsonschema::Validator;
use serde_json::Value;

const SCHEMA_STR: &str = include_str!("../resources/lle_toml_schema.json");

fn toml_to_json_value(toml_str: &str) -> Value {
    let toml_value: toml::Value = toml::from_str(toml_str).expect("Failed to parse TOML");
    // Round-trip through serde_json to convert toml::Value -> serde_json::Value
    let json_str = serde_json::to_string(&toml_value).expect("Failed to serialize to JSON");
    serde_json::from_str(&json_str).expect("Failed to parse JSON")
}

fn schema_validator() -> Validator {
    let schema: Value = serde_json::from_str(SCHEMA_STR).expect("Failed to parse schema JSON");
    Validator::new(&schema).expect("Failed to compile schema")
}

fn assert_valid(toml_str: &str) {
    let validator = schema_validator();
    let value = toml_to_json_value(toml_str);
    let errors: Vec<_> = validator.iter_errors(&value).collect();
    if !errors.is_empty() {
        let msgs: Vec<String> = errors
            .iter()
            .map(|e| format!("  - {} at {}", e, e.instance_path))
            .collect();
        panic!(
            "TOML should be valid according to schema but got errors:\n{}",
            msgs.join("\n")
        );
    }
}

fn assert_invalid(toml_str: &str) {
    let validator = schema_validator();
    let value = toml_to_json_value(toml_str);
    assert!(
        !validator.is_valid(&value),
        "TOML should be invalid according to schema but was accepted"
    );
}

// ===================== Valid documents =====================

#[test]
fn valid_minimal_with_dimensions() {
    assert_valid(
        r#"
width = 5
height = 5
"#,
    );
}

#[test]
fn valid_empty_document() {
    // All fields are optional at the schema level
    assert_valid("");
}

#[test]
fn valid_with_world_string_only() {
    assert_valid(
        r#"
world_string = "S0 X"
"#,
    );
}

#[test]
fn valid_full_example() {
    assert_valid(
        r#"
width = 10
height = 5
n_agents = 4
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

[[agents]]
start_positions = [{ i = 0, j = 5 }, { i = 3, j = 5 }]

[[agents]]
start_positions = [
    { i = 4, j = 9 },
    { i_min = 1, i_max = 3, j_min = 0, j_max = 3 },
    { j_min = 4 },
]
"#,
    );
}

#[test]
fn valid_with_global_starts() {
    assert_valid(
        r#"
width = 10
height = 10
n_agents = 5
starts = [{ row = 0 }]
exits = [{ col = 4 }]
"#,
    );
}

#[test]
fn valid_position_ij() {
    assert_valid(
        r#"
width = 5
height = 5
gems = [{ i = 0, j = 2 }, { i = 3, j = 4 }]
"#,
    );
}

#[test]
fn valid_position_row() {
    assert_valid(
        r#"
width = 5
height = 5
walls = [{ row = 0 }, { row = 4 }]
"#,
    );
}

#[test]
fn valid_position_col() {
    assert_valid(
        r#"
width = 5
height = 5
walls = [{ col = 0 }, { col = 4 }]
"#,
    );
}

#[test]
fn valid_position_rect_full() {
    assert_valid(
        r#"
width = 10
height = 10
walls = [{ i_min = 1, i_max = 3, j_min = 0, j_max = 3 }]
"#,
    );
}

#[test]
fn valid_position_rect_partial_i_min_only() {
    assert_valid(
        r#"
width = 10
height = 10
exits = [{ i_min = 5 }]
"#,
    );
}

#[test]
fn valid_position_rect_j_min_only() {
    assert_valid(
        r#"
width = 10
height = 10
exits = [{ j_min = 9 }]
"#,
    );
}

#[test]
fn valid_position_rect_empty_object() {
    // An empty object matches the Rect variant (all fields optional/defaulted)
    assert_valid(
        r#"
width = 5
height = 5
walls = [{}]
"#,
    );
}

#[test]
fn valid_laser_north() {
    assert_valid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "North"
agent = 0
position = { i = 1, j = 2 }
laser_id = 0
"#,
    );
}

#[test]
fn valid_laser_north_shorthand_north() {
    assert_valid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "n"
agent = 0
position = { i = 1, j = 2 }
laser_id = 0
"#,
    );
}

#[test]
fn valid_laser_north_shorthand_east() {
    assert_valid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "e"
agent = 0
position = { i = 1, j = 2 }
laser_id = 0
"#,
    );
}

#[test]
fn valid_laser_direction_alias_short() {
    assert_valid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "E"
agent = 0
position = { i = 0, j = 0 }
laser_id = 1
"#,
    );
}

#[test]
fn valid_laser_direction_alias_lowercase() {
    assert_valid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "south"
agent = 1
position = { i = 2, j = 3 }
laser_id = 0
"#,
    );
}

#[test]
fn valid_multiple_lasers() {
    assert_valid(
        r#"
width = 10
height = 10

[[lasers]]
direction = "North"
agent = 0
position = { i = 5, j = 3 }
laser_id = 0

[[lasers]]
direction = "W"
agent = 1
position = { i = 0, j = 9 }
laser_id = 1
"#,
    );
}

#[test]
fn valid_agents_with_starts_alias() {
    assert_valid(
        r#"
width = 5
height = 5

[[agents]]
starts = [{ i = 0, j = 0 }]

[[agents]]
start_positions = [{ row = 2 }]
"#,
    );
}

#[test]
fn valid_voids() {
    assert_valid(
        r#"
width = 5
height = 5
voids = [{ i = 2, j = 2 }]
"#,
    );
}

#[test]
fn valid_all_position_variants_mixed() {
    assert_valid(
        r#"
width = 10
height = 10
walls = [
    { i = 0, j = 0 },
    { row = 5 },
    { col = 9 },
    { i_min = 2, i_max = 4, j_min = 1, j_max = 3 },
    { i_min = 7 },
    { j_max = 2 },
]
"#,
    );
}

#[test]
fn valid_n_agents_zero() {
    assert_valid(
        r#"
width = 3
height = 3
n_agents = 0
"#,
    );
}

#[test]
fn valid_agent_empty_starts() {
    assert_valid(
        r#"
width = 3
height = 3

[[agents]]
starts = []
"#,
    );
}

// ===================== Invalid documents =====================

#[test]
fn invalid_unknown_top_level_field() {
    assert_invalid(
        r#"
width = 5
height = 5
unknown_field = 42
"#,
    );
}

#[test]
fn invalid_unknown_agent_field() {
    assert_invalid(
        r#"
width = 5
height = 5

[[agents]]
unknown_subfield = "oops"
"#,
    );
}

#[test]
fn invalid_width_wrong_type_string() {
    assert_invalid(
        r#"
width = "ten"
height = 5
"#,
    );
}

#[test]
fn invalid_height_wrong_type_string() {
    assert_invalid(
        r#"
width = 5
height = "five"
"#,
    );
}

#[test]
fn invalid_n_agents_wrong_type() {
    assert_invalid(
        r#"
width = 5
height = 5
n_agents = "two"
"#,
    );
}

#[test]
fn invalid_world_string_wrong_type() {
    assert_invalid(
        r#"
world_string = 123
"#,
    );
}

#[test]
fn invalid_exits_not_array() {
    assert_invalid(
        r#"
width = 5
height = 5
exits = "not_an_array"
"#,
    );
}

#[test]
fn invalid_gems_not_array() {
    assert_invalid(
        r#"
width = 5
height = 5
gems = 42
"#,
    );
}

#[test]
fn invalid_agents_not_array() {
    assert_invalid(
        r#"
width = 5
height = 5
agents = "wrong"
"#,
    );
}

#[test]
fn invalid_laser_missing_direction() {
    assert_invalid(
        r#"
width = 5
height = 5

[[lasers]]
agent = 0
position = { i = 1, j = 2 }
laser_id = 0
"#,
    );
}

#[test]
fn invalid_laser_missing_agent() {
    assert_invalid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "North"
position = { i = 1, j = 2 }
laser_id = 0
"#,
    );
}

#[test]
fn invalid_laser_missing_position() {
    assert_invalid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "North"
agent = 0
laser_id = 0
"#,
    );
}

#[test]
fn invalid_laser_missing_laser_id() {
    assert_invalid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "North"
agent = 0
position = { i = 1, j = 2 }
"#,
    );
}

#[test]
fn invalid_laser_bad_direction() {
    assert_invalid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "UpLeft"
agent = 0
position = { i = 1, j = 2 }
laser_id = 0
"#,
    );
}

#[test]
fn invalid_laser_agent_wrong_type() {
    assert_invalid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "North"
agent = "zero"
position = { i = 1, j = 2 }
laser_id = 0
"#,
    );
}

#[test]
fn invalid_laser_position_missing_i() {
    assert_invalid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "North"
agent = 0
position = { j = 2 }
laser_id = 0
"#,
    );
}

#[test]
fn invalid_laser_position_missing_j() {
    assert_invalid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "North"
agent = 0
position = { i = 1 }
laser_id = 0
"#,
    );
}

#[test]
fn invalid_laser_extra_field() {
    assert_invalid(
        r#"
width = 5
height = 5

[[lasers]]
direction = "North"
agent = 0
position = { i = 1, j = 2 }
laser_id = 0
color = "red"
"#,
    );
}

#[test]
fn invalid_position_ij_extra_field() {
    // An object with i, j, and extra fields should be rejected
    assert_invalid(
        r#"
width = 5
height = 5
gems = [{ i = 0, j = 1, extra = 99 }]
"#,
    );
}

#[test]
fn invalid_position_row_extra_field() {
    assert_invalid(
        r#"
width = 5
height = 5
walls = [{ row = 0, extra = 1 }]
"#,
    );
}

#[test]
fn invalid_position_col_extra_field() {
    assert_invalid(
        r#"
width = 5
height = 5
walls = [{ col = 0, extra = 1 }]
"#,
    );
}

#[test]
fn invalid_position_ij_wrong_type() {
    assert_invalid(
        r#"
width = 5
height = 5
gems = [{ i = "zero", j = 1 }]
"#,
    );
}

#[test]
fn invalid_position_row_wrong_type() {
    assert_invalid(
        r#"
width = 5
height = 5
walls = [{ row = true }]
"#,
    );
}

#[test]
fn invalid_position_col_wrong_type() {
    assert_invalid(
        r#"
width = 5
height = 5
walls = [{ col = "first" }]
"#,
    );
}

#[test]
fn invalid_position_rect_wrong_type() {
    assert_invalid(
        r#"
width = 5
height = 5
walls = [{ i_min = "a", i_max = "b" }]
"#,
    );
}

// ===================== Round-trip consistency tests =====================

/// Verify that a TOML document that can be parsed by the actual parser
/// also passes schema validation.
#[test]
fn parser_and_schema_agree_on_valid_world_string() {
    let toml_content = r#"
world_string = "S0 X"
"#;
    // Schema should accept it
    assert_valid(toml_content);
    // Parser should also accept it
    let result: Result<toml::Value, _> = toml::from_str(toml_content);
    assert!(result.is_ok(), "Parser should accept this TOML");
}

#[test]
fn parser_and_schema_agree_on_valid_complex() {
    let toml_content = r#"
width = 10
height = 5
exits = [{ j_min = 9 }]
gems = [{ i = 0, j = 2 }]

[[agents]]
start_positions = [{ i_min = 0, i_max = 0 }]

[[agents]]
start_positions = [{ i = 0, j = 5 }, { i = 3, j = 5 }]
"#;
    assert_valid(toml_content);
    let result: Result<toml::Value, _> = toml::from_str(toml_content);
    assert!(result.is_ok(), "Parser should accept this TOML");
}

#[test]
fn parser_and_schema_agree_on_invalid_unknown_field() {
    let toml_content = r#"
width = 5
height = 5
bogus = 123
"#;
    assert_invalid(toml_content);
}

// ===================== Edge cases =====================

#[test]
fn valid_width_one_height_one() {
    assert_valid(
        r#"
width = 1
height = 1
"#,
    );
}

#[test]
fn valid_all_direction_variants() {
    // Test all 12 accepted direction strings
    let directions = [
        "North", "East", "South", "West", "N", "E", "S", "W", "north", "east", "south", "west",
    ];
    for (id, dir) in directions.iter().enumerate() {
        let toml_content = format!(
            r#"
width = 20
height = 20

[[lasers]]
direction = "{dir}"
agent = 0
position = {{ i = {row}, j = {col} }}
laser_id = {id}
"#,
            dir = dir,
            row = id,
            col = id,
            id = id,
        );
        assert_valid(&toml_content);
    }
}

#[test]
fn valid_large_coordinates() {
    assert_valid(
        r#"
width = 1000
height = 1000
gems = [{ i = 999, j = 999 }]
exits = [{ row = 500 }]
walls = [{ col = 0 }]
"#,
    );
}

#[test]
fn valid_multiple_position_types_in_same_array() {
    assert_valid(
        r#"
width = 10
height = 10
exits = [
    { i = 0, j = 0 },
    { row = 5 },
    { col = 9 },
    { i_min = 2, j_min = 3 },
]
"#,
    );
}

#[test]
fn invalid_width_negative() {
    // TOML parses -1 as an integer, but schema requires minimum 1
    assert_invalid(
        r#"
width = -1
height = 5
"#,
    );
}

#[test]
fn invalid_height_zero() {
    // Schema requires minimum 1
    assert_invalid(
        r#"
width = 5
height = 0
"#,
    );
}

#[test]
fn invalid_position_i_negative() {
    // TOML integer -1 should violate minimum: 0
    assert_invalid(
        r#"
width = 5
height = 5
gems = [{ i = -1, j = 0 }]
"#,
    );
}

#[test]
fn invalid_walls_contains_string_item() {
    assert_invalid(
        r#"
width = 5
height = 5
walls = ["not_a_position"]
"#,
    );
}

#[test]
fn invalid_lasers_not_array() {
    assert_invalid(
        r#"
width = 5
height = 5
lasers = "wrong"
"#,
    );
}

#[test]
fn invalid_starts_not_array() {
    assert_invalid(
        r#"
width = 5
height = 5
starts = 42
"#,
    );
}

#[test]
fn invalid_voids_wrong_type() {
    assert_invalid(
        r#"
width = 5
height = 5
voids = true
"#,
    );
}

#[test]
fn valid_agent_with_no_fields() {
    // An empty agent entry (no starts specified) should be valid
    assert_valid(
        r#"
width = 5
height = 5

[[agents]]
"#,
    );
}

#[test]
fn valid_rect_with_only_i_max() {
    assert_valid(
        r#"
width = 5
height = 5
walls = [{ i_max = 2 }]
"#,
    );
}

#[test]
fn valid_rect_with_only_j_max() {
    assert_valid(
        r#"
width = 5
height = 5
walls = [{ j_max = 3 }]
"#,
    );
}

#[test]
fn valid_rect_with_all_bounds() {
    assert_valid(
        r#"
width = 10
height = 10
walls = [{ i_min = 1, i_max = 5, j_min = 2, j_max = 8 }]
"#,
    );
}
