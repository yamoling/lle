use crate::World;

use super::problem_state::ProblemState;

pub struct SearchProblem {
    world: World,
    initial_state: ProblemState,
    goal_state: ProblemState,
}

impl SearchProblem {
    pub fn new(world: World) -> Self {
        let initial_state = ProblemState::new(
            world.agent_positions().clone(),
            vec![false; world.n_gems() as usize],
        );
        let goal_state = ProblemState::new(
            world.agent_positions().clone(),
            vec![true; world.n_gems() as usize],
        );
        Self {
            world,
            initial_state,
            goal_state,
        }
    }

    pub fn initial_state(&self) -> &ProblemState {
        &self.initial_state
    }

    pub fn goal_state(&self) -> &ProblemState {
        &self.goal_state
    }

    pub fn world(&self) -> &World {
        &self.world
    }
}

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct Point {
    x: usize,
    y: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Node {
    point: Point,
    g: usize,
    h: usize,
}

impl Node {
    fn new(point: Point, g: usize, h: usize) -> Node {
        Node { point, g, h }
    }

    fn f(&self) -> usize {
        self.g + self.h
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f().cmp(&self.f())
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn astar_search(
    start: Point,
    goal: Point,
    is_valid: &dyn Fn(Point) -> bool,
    heuristic: &dyn Fn(Point, Point) -> usize,
) -> Option<Vec<Point>> {
    let mut open_set = BinaryHeap::new();
    let mut came_from = HashMap::new();
    let mut g_scores = HashMap::new();

    open_set.push(Node::new(start, 0, heuristic(start, goal)));
    g_scores.insert(start, 0);

    while let Some(current) = open_set.pop() {
        if current.point == goal {
            // Reconstruct the path
            let mut path = vec![current.point];
            let mut node = current;

            while let Some(&parent) = came_from.get(&node.point) {
                path.push(parent);
                node = parent;
            }

            path.reverse();
            return Some(path);
        }

        for &next_point in &[
            Point {
                x: current.point.x + 1,
                y: current.point.y,
            },
            Point {
                x: current.point.x - 1,
                y: current.point.y,
            },
            Point {
                x: current.point.x,
                y: current.point.y + 1,
            },
            Point {
                x: current.point.x,
                y: current.point.y - 1,
            },
        ] {
            if !is_valid(next_point) {
                continue;
            }

            let tentative_g_score = g_scores[&current.point] + 1;

            if !g_scores.contains_key(&next_point) || tentative_g_score < g_scores[&next_point] {
                g_scores.insert(next_point, tentative_g_score);
                let h = heuristic(next_point, goal);
                let f = tentative_g_score + h;
                open_set.push(Node::new(next_point, tentative_g_score, h));
                came_from.insert(next_point, current.point);
            }
        }
    }

    None // No path found
}
