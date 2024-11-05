use crate::Position;

#[test]
fn test_sample_different() {
    let mut rng = rand::thread_rng();
    let random_start_positions = vec![
        vec![(0, 0).into()],
        vec![(1, 0).into()],
        vec![(2, 0).into()],
    ];
    let result = super::sample_different(&mut rng, &random_start_positions);
    assert_eq!(result, vec![(0, 0), (1, 0), (2, 0)]);
}

#[test]
fn test_sample_different_deterministic() {
    let mut rng = rand::thread_rng();
    let random_start_positions = vec![
        vec![Position { i: 0, j: 0 }],
        vec![Position { i: 0, j: 0 }, Position { i: 1, j: 0 }],
        vec![
            Position { i: 0, j: 0 },
            Position { i: 1, j: 0 },
            (2, 0).into(),
        ],
    ];
    let result = super::sample_different(&mut rng, &random_start_positions);
    assert_eq!(result, vec![(0, 0), (1, 0), (2, 0)]);
}

#[test]
#[should_panic]
fn test_sample_different_impossible() {
    let mut rng = rand::thread_rng();
    let random_start_positions = vec![
        vec![Position { i: 0, j: 0 }],
        vec![Position { i: 0, j: 1 }],
        vec![Position { i: 0, j: 2 }],
        vec![Position { i: 0, j: 0 }, (0, 1).into(), (0, 2).into()],
    ];
    super::sample_different(&mut rng, &random_start_positions);
}
