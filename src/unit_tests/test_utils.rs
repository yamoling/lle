use crate::Position;

#[test]
fn test_sample_different() {
    let mut rng = rand::rng();
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
    let mut rng = rand::rng();
    let random_start_positions = vec![
        vec![Position::new2d(0, 0)],
        vec![Position::new2d(0, 0), Position::new2d(1, 0)],
        vec![Position::new2d(0, 0), Position::new2d(1, 0), (2, 0).into()],
    ];
    let result = super::sample_different(&mut rng, &random_start_positions);
    assert_eq!(result, vec![(0, 0), (1, 0), (2, 0)]);
}

#[test]
#[should_panic]
fn test_sample_different_impossible() {
    let mut rng = rand::rng();
    let random_start_positions = vec![
        vec![Position::new2d(0, 0)],
        vec![Position::new2d(0, 1)],
        vec![Position::new2d(0, 2)],
        vec![Position::new2d(0, 0), (0, 1).into(), (0, 2).into()],
    ];
    super::sample_different(&mut rng, &random_start_positions);
}

#[test]
fn test_into_equality() {
    let pos: Position = (1, 2, 0).into();
    assert_eq!(pos.i, 1);
    assert_eq!(pos.j, 2);
    assert_eq!(pos.k, 0);
    let pos_2: Position = (1, 2).into();
    assert_eq!(pos_2, pos);
    let (i, j, k): (usize, usize, usize) = pos.into();
    assert_eq!(i, 1);
    assert_eq!(j, 2);
    assert_eq!(k, 0);
    let pos2: Position = (1, 2).into();
    assert_eq!(pos2, pos);
}
