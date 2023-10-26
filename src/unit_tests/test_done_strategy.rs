use crate::{
    world::{Core, DoneStrategy},
    Action,
};

#[test]
fn test_done_strategy_one_agent_exits() {
    let mut w = Core::try_from(
        "
        S0 S1 G
        X  X .
    ",
    )
    .unwrap();
    w.reset();
    assert!(!DoneStrategy::Competitive.is_done(&w));
    assert!(!DoneStrategy::Cooperarive.is_done(&w));
    w.step(&[Action::Stay, Action::South]).unwrap();
    assert!(DoneStrategy::Competitive.is_done(&w));
    assert!(!DoneStrategy::Cooperarive.is_done(&w));
}

#[test]
fn test_done_strategy_one_agent_dies_and_exits() {
    let mut w = Core::try_from(
        "
        S0 S1 G
        X  X  L0W
    ",
    )
    .unwrap();
    w.reset();
    assert!(!DoneStrategy::Competitive.is_done(&w));
    assert!(!DoneStrategy::Cooperarive.is_done(&w));
    w.step(&[Action::Stay, Action::South]).unwrap();
    assert!(!DoneStrategy::Competitive.is_done(&w));
    assert!(DoneStrategy::Cooperarive.is_done(&w));
}

#[test]
fn test_competitive_done_strategy_one_agent_dies() {
    let mut w = Core::try_from(
        "
        S0 S1 X
        X  .  L0W
    ",
    )
    .unwrap();
    w.reset();
    assert!(!DoneStrategy::Competitive.is_done(&w));
    w.step(&[Action::Stay, Action::South]).unwrap();
    assert!(!DoneStrategy::Competitive.is_done(&w));
}
