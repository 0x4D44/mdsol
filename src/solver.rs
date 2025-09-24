use std::collections::HashMap;
use std::time::{Duration, Instant};

/// ----- Card representation ----------------------------------------------------
/// We represent cards as u8 in [0, 51]. Suits are 0..=3; ranks are 0..=12 (Ace=0,...,King=12).
/// Mapping: 0..12 = Clubs A..K, 13..25 = Diamonds, 26..38 = Hearts, 39..51 = Spades.
#[inline]
fn rank(c: u8) -> u8 {
    c % 13
}

#[inline]
fn suit(c: u8) -> u8 {
    c / 13
}

#[inline]
fn is_red(s: u8) -> bool {
    s == 1 || s == 2
}

#[inline]
fn colors_alternate(a: u8, b: u8) -> bool {
    is_red(suit(a)) != is_red(suit(b))
}

/// Convenience for debugging (e.g., "AH", "TC").
#[allow(dead_code)]
fn card_str(c: u8) -> &'static str {
    const R: [&str; 13] = [
        "A", "2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K",
    ];
    R[rank(c) as usize]
}

/// ----- Game state -------------------------------------------------------------
/// A tableau pile: `cards` is bottom->top; `up_from` is the index of the first face-up card.
#[derive(Clone)]
struct Pile {
    cards: Vec<u8>,
    up_from: usize,
}

impl Pile {
    #[inline]
    fn top(&self) -> Option<u8> {
        self.cards.last().copied()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

/// K+ compressed stock/waste representation.
#[derive(Clone)]
struct KPlus {
    stock: Vec<u8>,
    draw: u8,
    phase: u8,
}

impl KPlus {
    #[inline]
    fn playable_indices(&self) -> impl Iterator<Item = usize> + '_ {
        let d = self.draw as usize;
        let p = self.phase as usize;
        (0..self.stock.len()).filter(move |&i| i % d == p)
    }

    #[inline]
    fn take_at(&mut self, idx: usize) -> u8 {
        let c = self.stock.remove(idx);
        if self.draw > 1 {
            self.phase = (self.phase + self.draw - 1) % self.draw;
        }
        c
    }
}

/// Overall game state.
#[derive(Clone)]
struct State {
    piles: [Pile; 7],
    fnd: [i8; 4],
    k: KPlus,
}

impl State {
    fn normalize(&mut self) {
        loop {
            let mut progressed = false;

            for p in &mut self.piles {
                if p.up_from == p.cards.len() && p.up_from > 0 {
                    p.up_from -= 1;
                    progressed = true;
                }
            }

            'outer: loop {
                for i in 0..7 {
                    if let Some(top) = self.piles[i].top() {
                        let s = suit(top) as usize;
                        let need = self.fnd[s] + 1;
                        if need as u8 == rank(top) && safe_to_foundation(top, &self.fnd) {
                            self.piles[i].cards.pop();
                            if self.piles[i].up_from > self.piles[i].cards.len() {
                                self.piles[i].up_from = self.piles[i].cards.len();
                            }
                            self.fnd[s] += 1;
                            progressed = true;
                            continue 'outer;
                        }
                    }
                }

                let mut moved = false;
                let playable: Vec<usize> = self.k.playable_indices().collect();
                for idx in playable {
                    if idx >= self.k.stock.len() {
                        continue;
                    }
                    let c = self.k.stock[idx];
                    let s = suit(c) as usize;
                    if (self.fnd[s] + 1) as u8 == rank(c) && safe_to_foundation(c, &self.fnd) {
                        let card = self.k.take_at(idx);
                        debug_assert_eq!(card, c);
                        self.fnd[s] += 1;
                        progressed = true;
                        moved = true;
                        break;
                    }
                }
                if moved {
                    continue 'outer;
                }
                break;
            }

            if !progressed {
                break;
            }
        }
    }
}

fn safe_to_foundation(card: u8, fnd: &[i8; 4]) -> bool {
    let s = suit(card) as usize;
    let r = rank(card) as i8;
    let same_color = match s {
        0 => 3,
        3 => 0,
        1 => 2,
        2 => 1,
        _ => unreachable!(),
    };
    let (oc1, oc2) = if is_red(s as u8) {
        (0usize, 3usize)
    } else {
        (1usize, 2usize)
    };
    r <= (fnd[oc1].min(fnd[oc2]) + 2) && r <= (fnd[same_color] + 3)
}

#[inline]
fn can_build_onto(x: u8, y: u8) -> bool {
    rank(x) + 1 == rank(y) && colors_alternate(x, y)
}

#[inline]
fn is_king(c: u8) -> bool {
    rank(c) == 12
}

#[derive(Clone, Copy)]
enum Move {
    TableauToFoundation {
        src: usize,
    },
    TableauToTableau {
        src: usize,
        start_idx: usize,
        dst: usize,
    },
    WasteToFoundation {
        idx_in_k: usize,
    },
    WasteToTableau {
        idx_in_k: usize,
        dst: usize,
    },
    FoundationToTableau {
        suit: usize,
        dst: usize,
    },
}

fn generate_moves(s: &State) -> Vec<Move> {
    let mut moves: Vec<Move> = Vec::with_capacity(64);

    let expose_if_move = |p: &Pile, start_idx: usize| -> bool {
        start_idx == p.up_from && start_idx < p.cards.len()
    };

    for src in 0..7 {
        if let Some(c) = s.piles[src].top() {
            let su = suit(c) as usize;
            if (s.fnd[su] + 1) as u8 == rank(c) {
                moves.push(Move::TableauToFoundation { src });
            }
        }
    }

    for src in 0..7 {
        let p = &s.piles[src];
        if p.up_from >= p.cards.len() {
            continue;
        }
        let mut j = p.cards.len() - 1;
        let mut valid_starts: Vec<usize> = Vec::new();
        valid_starts.push(j);
        while j > p.up_from {
            let a = p.cards[j - 1];
            let b = p.cards[j];
            if can_build_onto(a, b) {
                j -= 1;
                valid_starts.push(j);
            } else {
                break;
            }
        }
        for &start_idx in &valid_starts {
            let bottom = p.cards[start_idx];
            for dst in 0..7 {
                if dst == src {
                    continue;
                }
                if let Some(t) = s.piles[dst].top() {
                    if can_build_onto(bottom, t) {
                        moves.push(Move::TableauToTableau {
                            src,
                            start_idx,
                            dst,
                        });
                    }
                }
            }
            if is_king(bottom) {
                if let Some(dst) = (0..7).find(|&i| s.piles[i].is_empty()) {
                    if dst != src {
                        moves.push(Move::TableauToTableau {
                            src,
                            start_idx,
                            dst,
                        });
                    }
                }
            }
        }
    }

    let k_idxs: Vec<usize> = s.k.playable_indices().collect();
    for idx in k_idxs {
        if idx >= s.k.stock.len() {
            continue;
        }
        let c = s.k.stock[idx];
        let su = suit(c) as usize;
        if (s.fnd[su] + 1) as u8 == rank(c) {
            moves.push(Move::WasteToFoundation { idx_in_k: idx });
        }
        for dst in 0..7 {
            if let Some(t) = s.piles[dst].top() {
                if can_build_onto(c, t) {
                    moves.push(Move::WasteToTableau { idx_in_k: idx, dst });
                }
            } else if is_king(c) {
                if let Some(dst0) = (0..7).find(|&i| s.piles[i].is_empty()) {
                    if dst0 == dst {
                        moves.push(Move::WasteToTableau { idx_in_k: idx, dst });
                    }
                }
            }
        }
    }

    for su in 0..4 {
        let r = s.fnd[su];
        if r < 0 {
            continue;
        }
        let c = (su as u8) * 13 + r as u8;
        let mut new_fnd = s.fnd;
        new_fnd[su] -= 1;
        if safe_to_foundation(c, &new_fnd) {
            continue;
        }
        for dst in 0..7 {
            if let Some(t) = s.piles[dst].top() {
                if can_build_onto(c, t) {
                    moves.push(Move::FoundationToTableau { suit: su, dst });
                }
            } else if is_king(c) {
                if let Some(dst0) = (0..7).find(|&i| s.piles[i].is_empty()) {
                    if dst0 == dst {
                        moves.push(Move::FoundationToTableau { suit: su, dst });
                    }
                }
            }
        }
    }

    moves.sort_by_key(|m| match *m {
        Move::TableauToTableau { src, start_idx, .. } => {
            if expose_if_move(&s.piles[src], start_idx) {
                0
            } else {
                3
            }
        }
        Move::WasteToTableau { .. } => 1,
        Move::WasteToFoundation { .. } => 2,
        Move::TableauToFoundation { .. } => 3,
        Move::FoundationToTableau { .. } => 4,
    });
    moves
}

fn apply_move(st: &mut State, mv: Move) {
    match mv {
        Move::TableauToFoundation { src } => {
            let c = st.piles[src].cards.pop().unwrap();
            let s = suit(c) as usize;
            st.fnd[s] += 1;
            if st.piles[src].up_from > st.piles[src].cards.len() {
                st.piles[src].up_from = st.piles[src].cards.len();
            }
        }
        Move::TableauToTableau {
            src,
            start_idx,
            dst,
        } => {
            let moving: Vec<u8> = st.piles[src].cards.drain(start_idx..).collect();
            st.piles[dst].cards.extend(moving);
            if st.piles[src].up_from > st.piles[src].cards.len() {
                st.piles[src].up_from = st.piles[src].cards.len();
            }
        }
        Move::WasteToFoundation { idx_in_k } => {
            let c = st.k.take_at(idx_in_k);
            let s = suit(c) as usize;
            st.fnd[s] += 1;
        }
        Move::WasteToTableau { idx_in_k, dst } => {
            let c = st.k.take_at(idx_in_k);
            st.piles[dst].cards.push(c);
        }
        Move::FoundationToTableau { suit: su, dst } => {
            let r = st.fnd[su] as u8;
            let c = (su as u8) * 13 + r;
            st.fnd[su] -= 1;
            st.piles[dst].cards.push(c);
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Hash)]
struct Key(u64);

fn hash_state(s: &State) -> Key {
    let mut h: u64 = 1469598103934665603;
    let mut mix = |x: u64| {
        h ^= x;
        h = h.wrapping_mul(1099511628211);
    };

    for &f in &s.fnd {
        mix((f as i64 as u64).wrapping_add(1));
    }
    mix(s.k.draw as u64);
    mix(s.k.phase as u64);
    mix(s.k.stock.len() as u64);
    for &c in &s.k.stock {
        mix(c as u64 + 0x9e3779b97f4a7c15);
    }
    for p in &s.piles {
        mix(0xA3);
        mix(p.up_from as u64);
        mix(p.cards.len() as u64);
        for &c in &p.cards {
            mix(c as u64);
        }
    }
    Key(h)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolveResult {
    Winnable,
    Unwinnable,
    Timeout,
}

struct Frame {
    state: State,
    key: Option<Key>,
    moves: Vec<Move>,
    next_child: usize,
    initialized: bool,
    found_success: bool,
}

impl Frame {
    fn new(state: State) -> Self {
        Self {
            state,
            key: None,
            moves: Vec::new(),
            next_child: 0,
            initialized: false,
            found_success: false,
        }
    }
}

fn dfs(
    start: State,
    tt: &mut HashMap<Key, bool>,
    deadline: Instant,
    node_counter: &mut u64,
) -> Option<bool> {
    let mut stack = vec![Frame::new(start)];

    while let Some(frame) = stack.last_mut() {
        if frame.initialized && frame.found_success {
            let key = frame.key.expect("initialized frames must have a key");
            tt.insert(key, true);
            stack.pop();
            if let Some(parent) = stack.last_mut() {
                parent.found_success = true;
            } else {
                return Some(true);
            }
            continue;
        }

        if !frame.initialized {
            frame.state.normalize();

            if frame.state.fnd.iter().all(|&r| r == 12) {
                let key = hash_state(&frame.state);
                tt.insert(key, true);
                stack.pop();
                if let Some(parent) = stack.last_mut() {
                    parent.found_success = true;
                } else {
                    return Some(true);
                }
                continue;
            }

            *node_counter += 1;
            if (*node_counter & 0x3ff) == 0 && Instant::now() >= deadline {
                return None;
            }

            let key = hash_state(&frame.state);
            frame.key = Some(key);

            if let Some(&res) = tt.get(&key) {
                stack.pop();
                if let Some(parent) = stack.last_mut() {
                    if res {
                        parent.found_success = true;
                    }
                } else {
                    return Some(res);
                }
                continue;
            }

            frame.moves = generate_moves(&frame.state);
            frame.next_child = 0;
            frame.initialized = true;

            if frame.moves.is_empty() {
                tt.insert(key, false);
                stack.pop();
                if stack.is_empty() {
                    return Some(false);
                }
                continue;
            }
        }

        if frame.next_child < frame.moves.len() {
            let mv = frame.moves[frame.next_child];
            frame.next_child += 1;

            let mut child_state = frame.state.clone();
            apply_move(&mut child_state, mv);
            stack.push(Frame::new(child_state));
            continue;
        }

        let key = frame.key.expect("initialized frames must have a key");
        let result = frame.found_success;
        tt.insert(key, result);
        stack.pop();
        if let Some(parent) = stack.last_mut() {
            if result {
                parent.found_success = true;
            }
        } else {
            return Some(result);
        }
    }

    Some(false)
}

pub fn solve_deck(deck: &[u8; 52], draw_size: u8, time_budget: Duration) -> SolveResult {
    assert!(draw_size == 1 || draw_size == 3, "draw_size must be 1 or 3");

    let mut it = 0usize;
    let mut piles: [Pile; 7] = std::array::from_fn(|_| Pile {
        cards: Vec::new(),
        up_from: 0,
    });
    for (i, pile) in piles.iter_mut().enumerate() {
        for _ in 0..=i {
            pile.cards.push(deck[it]);
            it += 1;
        }
        pile.up_from = pile.cards.len() - 1;
    }

    let stock: Vec<u8> = deck[it..].to_vec();
    let phase = if draw_size == 0 { 0 } else { draw_size - 1 };
    let k = KPlus {
        stock,
        draw: draw_size,
        phase,
    };

    let mut s = State {
        piles,
        fnd: [-1; 4],
        k,
    };
    s.normalize();

    let start = Instant::now();
    let deadline = start
        .checked_add(time_budget)
        .unwrap_or_else(|| start + Duration::from_secs(5));
    let mut tt: HashMap<Key, bool> = HashMap::with_capacity(1 << 16);
    let mut nodes: u64 = 0;
    match dfs(s, &mut tt, deadline, &mut nodes) {
        Some(true) => SolveResult::Winnable,
        Some(false) => SolveResult::Unwinnable,
        None => SolveResult::Timeout,
    }
}

#[allow(dead_code)]
pub fn parse_deck(tokens: &[&str]) -> Option<[u8; 52]> {
    if tokens.len() != 52 {
        return None;
    }
    fn parse_card(tok: &str) -> Option<u8> {
        let t = tok.trim().to_ascii_uppercase();
        let bytes = t.as_bytes();
        if bytes.len() < 2 || bytes.len() > 3 {
            return None;
        }
        let r = match bytes[0] {
            b'A' => 0,
            b'2' => 1,
            b'3' => 2,
            b'4' => 3,
            b'5' => 4,
            b'6' => 5,
            b'7' => 6,
            b'8' => 7,
            b'9' => 8,
            b'T' => 9,
            b'J' => 10,
            b'Q' => 11,
            b'K' => 12,
            _ => return None,
        };
        let s = match bytes[bytes.len() - 1] {
            b'C' => 0,
            b'D' => 1,
            b'H' => 2,
            b'S' => 3,
            _ => return None,
        };
        Some(s * 13 + r)
    }
    let mut out = [0u8; 52];
    for (i, &tok) in tokens.iter().enumerate() {
        out[i] = parse_card(tok)?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_to_foundation_basics() {
        let f = [-1; 4];
        assert!(safe_to_foundation(0, &f));
        assert!(safe_to_foundation(13 + 1, &f));
        assert!(!safe_to_foundation(26 + 4, &f));
    }

    #[test]
    fn test_kplus_indices() {
        let mut k = KPlus {
            stock: (0..24u8).collect(),
            draw: 3,
            phase: 2,
        };
        let idxs: Vec<_> = k.playable_indices().collect();
        assert!(idxs.iter().all(|&i| i % 3 == 2));
        let first = idxs[0];
        let c = k.take_at(first);
        assert_eq!(c, 2);
        assert_eq!(k.phase, 1);
    }

    #[test]
    fn test_solve_trivial() {
        let mut deck = [0u8; 52];
        for (i, slot) in deck.iter_mut().enumerate() {
            *slot = i as u8;
        }
        let res = solve_deck(&deck, 1, Duration::from_millis(200));
        assert!(matches!(
            res,
            SolveResult::Winnable | SolveResult::Timeout | SolveResult::Unwinnable
        ));
    }
}
