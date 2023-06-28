use lle::{Action, World, WorldError};

#[test]
fn test_available_actions() {
    let w = World::try_from(
        "@ @ L0S @  @
             @ .  .  .  @
             @ F  S0 .  @
             @ .  .  .  @
             @ @  @  @  @",
    )
    .unwrap();
    let actions = w.available_actions();
    assert_eq!(actions[0].len(), 5);
}

#[test]
fn test_available_actions_two_agents() {
    let w = World::try_from(
        "
        F  .  .  
        F  S0 S1 
        .  .  .  
        ",
    )
    .unwrap();
    let actions = w.available_actions();
    assert_eq!(actions[0].len(), 4);
    assert!(actions[0].contains(&Action::North));
    assert!(actions[0].contains(&Action::South));
    assert!(actions[0].contains(&Action::Stay));
    assert!(actions[0].contains(&Action::West));

    assert_eq!(actions[1].len(), 3);
    assert!(actions[1].contains(&Action::North));
    assert!(actions[1].contains(&Action::South));
    assert!(actions[1].contains(&Action::Stay));
}

#[test]
fn parse_empty_world() {
    match World::try_from("") {
        Ok(_) => panic!("Should not be able to parse empty world"),
        Err(e) => match e {
            WorldError::EmptyWorld { .. } => {}
            _ => panic!("Expected EmptyWorld, got {e:?}"),
        },
    }
}

#[test]
fn parse_inconsistent_row_lengths() {
    match World::try_from(
        "F S0 .
         . .",
    ) {
        Ok(_) => panic!("Should not be able to parse worlds with inconsistent row lengths"),
        Err(e) => match e {
            WorldError::InconsistentDimensions {
                actual_n_cols,
                expected_n_cols,
                row,
            } => {
                assert_eq!(actual_n_cols, 2);
                assert_eq!(expected_n_cols, 3);
                assert_eq!(row, 1);
            }
            _ => panic!("Expected InconsistentDimensions, got {e:?}"),
        },
    }
}

#[test]
fn parse_inconsistent_start_exit_tiles() {
    match World::try_from("F S0 F") {
        Ok(_) => panic!("Should not be able to parse worlds with #exit != #start"),
        Err(e) => match e {
            WorldError::InconsistentNumberOfAgents {
                n_exit_pos,
                n_start_pos,
            } => {
                assert_eq!(n_start_pos, 1);
                assert_eq!(n_exit_pos, 2);
            }
            _ => panic!("Expected InconsistentNumberOfAgents, got {e:?}"),
        },
    }
}

#[test]
fn parse_no_agents() {
    match World::try_from(". . .") {
        Ok(_) => panic!("Should not be able to create worlds without agents"),
        Err(e) => match e {
            WorldError::NoAgents => {}
            _ => panic!("Expected NoAgents, got {e:?}"),
        },
    }
}

#[test]
fn test_move_end_game() {
    let mut w = World::try_from(
        "
    S0 F .
    .  . .
    .  . .",
    )
    .unwrap();
    w.step(&[Action::South]);
    assert!(!w.done());
    w.step(&[Action::South]);
    assert!(!w.done());
    w.step(&[Action::East]);
    assert!(!w.done());
    w.step(&[Action::North]);
    assert!(!w.done());
    w.step(&[Action::North]);
    assert!(w.done());
}

#[test]
fn test_reward_finish() {
    let mut w = World::try_from(
        "
    S0 F .
    .  . .
    .  . .",
    )
    .unwrap();
    let r = w.step(&[Action::East]);
    assert_eq!(r, 2);
}
