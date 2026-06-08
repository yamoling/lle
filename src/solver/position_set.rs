use crate::Position;

/// A dense bitset over every `(i, j)` position of a `height x width` grid, indexed as
/// `i * width + j`. Used in place of `HashSet<Position>` in the solver's hot paths
/// (reachability caches, exit-reachability), where positions are dense, bounded, and
/// repeatedly intersected/iterated — operations a word-at-a-time bitset handles far more
/// cheaply than hashing individual `Position`s.
#[derive(Clone, Debug)]
pub struct PositionSet {
    width: usize,
    words: Vec<u64>,
}

#[inline]
fn word_and_bit(width: usize, pos: &Position) -> (usize, u32) {
    let idx = pos.i * width + pos.j;
    (idx / 64, (idx % 64) as u32)
}

#[inline]
fn position_of(width: usize, word_idx: usize, bit: u32) -> Position {
    let idx = word_idx * 64 + bit as usize;
    Position {
        i: idx / width,
        j: idx % width,
    }
}

/// Pulls the next set bit out of a word stream, mutating `word_idx`/`current` in place.
/// Shared by the borrowing and owning iterators (and the lazy intersection) below.
#[inline]
fn advance(words: &[u64], word_idx: &mut usize, current: &mut u64) -> Option<(usize, u32)> {
    while *current == 0 {
        if *word_idx >= words.len() {
            return None;
        }
        *current = words[*word_idx];
        *word_idx += 1;
    }
    let bit = current.trailing_zeros();
    *current &= *current - 1;
    Some((*word_idx - 1, bit))
}

impl PositionSet {
    /// An empty set over a `height x width` grid.
    pub fn empty(height: usize, width: usize) -> Self {
        PositionSet {
            width,
            words: vec![0u64; (height * width).div_ceil(64)],
        }
    }

    /// A set containing only `pos`, over a `height x width` grid.
    pub fn singleton(height: usize, width: usize, pos: Position) -> Self {
        let mut set = Self::empty(height, width);
        set.insert(pos);
        set
    }

    pub fn insert(&mut self, pos: Position) {
        let (word, bit) = word_and_bit(self.width, &pos);
        self.words[word] |= 1u64 << bit;
    }

    pub fn remove(&mut self, pos: &Position) {
        let (word, bit) = word_and_bit(self.width, pos);
        self.words[word] &= !(1u64 << bit);
    }

    pub fn contains(&self, pos: &Position) -> bool {
        let (word, bit) = word_and_bit(self.width, pos);
        self.words[word] & (1u64 << bit) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    pub fn size(&self) -> usize {
        self.words.iter().map(|&w| w.count_ones() as usize).sum()
    }

    /// Intersect this set with `other` in place (`self &= other`), word at a time.
    pub fn intersect_with(&mut self, other: &PositionSet) {
        for (a, &b) in self.words.iter_mut().zip(&other.words) {
            *a &= b;
        }
    }

    /// Remove every position in `other` from this set in place (`self -= other`).
    pub fn subtract(&mut self, other: &PositionSet) {
        for (a, &b) in self.words.iter_mut().zip(&other.words) {
            *a &= !b;
        }
    }

    /// Drop every position for which `f` returns `false`.
    pub fn retain(&mut self, mut f: impl FnMut(&Position) -> bool) {
        for (word_idx, word) in self.words.iter_mut().enumerate() {
            let mut remaining = *word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros();
                remaining &= remaining - 1;
                let pos = position_of(self.width, word_idx, bit);
                if !f(&pos) {
                    *word &= !(1u64 << bit);
                }
            }
        }
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            width: self.width,
            words: &self.words,
            word_idx: 0,
            current: 0,
        }
    }

    /// Lazily iterate the positions present in both `self` and `other`, without allocating
    /// an intermediate set.
    pub fn intersection<'a>(&'a self, other: &'a PositionSet) -> Intersection<'a> {
        Intersection {
            width: self.width,
            a: &self.words,
            b: &other.words,
            word_idx: 0,
            current: 0,
        }
    }
}

pub struct Iter<'a> {
    width: usize,
    words: &'a [u64],
    word_idx: usize,
    current: u64,
}

impl Iterator for Iter<'_> {
    type Item = Position;

    fn next(&mut self) -> Option<Position> {
        let (word_idx, bit) = advance(self.words, &mut self.word_idx, &mut self.current)?;
        Some(position_of(self.width, word_idx, bit))
    }
}

pub struct IntoIter {
    width: usize,
    words: Vec<u64>,
    word_idx: usize,
    current: u64,
}

impl Iterator for IntoIter {
    type Item = Position;

    fn next(&mut self) -> Option<Position> {
        let (word_idx, bit) = advance(&self.words, &mut self.word_idx, &mut self.current)?;
        Some(position_of(self.width, word_idx, bit))
    }
}

impl IntoIterator for PositionSet {
    type Item = Position;
    type IntoIter = IntoIter;

    fn into_iter(self) -> IntoIter {
        IntoIter {
            width: self.width,
            words: self.words,
            word_idx: 0,
            current: 0,
        }
    }
}

impl<'a> IntoIterator for &'a PositionSet {
    type Item = Position;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

pub struct Intersection<'a> {
    width: usize,
    a: &'a [u64],
    b: &'a [u64],
    word_idx: usize,
    current: u64,
}

impl Iterator for Intersection<'_> {
    type Item = Position;

    fn next(&mut self) -> Option<Position> {
        loop {
            if self.current != 0 {
                let bit = self.current.trailing_zeros();
                self.current &= self.current - 1;
                return Some(position_of(self.width, self.word_idx - 1, bit));
            }
            if self.word_idx >= self.a.len() {
                return None;
            }
            self.current = self.a[self.word_idx] & self.b[self.word_idx];
            self.word_idx += 1;
        }
    }
}
