//! Core Solitaire game engine scaffolding.
//! Implements deck construction, shuffling via BCrypt RNG, and a fresh deal.

use anyhow::{anyhow, Result};
use std::time::Duration;

use crate::solver::{solve_deck, SolveResult};
use windows::Win32::Foundation::STATUS_SUCCESS;
use windows::Win32::Security::Cryptography::{
    BCryptGenRandom, BCRYPT_ALG_HANDLE, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
};

const FOUNDATION_PILES: usize = 4;
const TABLEAU_PILES: usize = 7;
const DECK_SIZE: usize = 52;
const SOLVER_TIME_BUDGET_MS: u64 = 120;
const SUITS: [Suit; 4] = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
const RANKS: [Rank; 13] = [
    Rank::Ace,
    Rank::Two,
    Rank::Three,
    Rank::Four,
    Rank::Five,
    Rank::Six,
    Rank::Seven,
    Rank::Eight,
    Rank::Nine,
    Rank::Ten,
    Rank::Jack,
    Rank::Queen,
    Rank::King,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardColor {
    Red,
    Black,
}

impl Suit {
    pub const fn row(self) -> u8 {
        match self {
            Suit::Spades => 0,
            Suit::Hearts => 1,
            Suit::Diamonds => 2,
            Suit::Clubs => 3,
        }
    }

    pub const fn color(self) -> CardColor {
        match self {
            Suit::Hearts | Suit::Diamonds => CardColor::Red,
            Suit::Spades | Suit::Clubs => CardColor::Black,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    Ace = 1,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    pub const fn column(self) -> u8 {
        (self as u8) - 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
    pub face_up: bool,
    pub sprite_index: u8,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        let sprite_index = suit.row() * 13 + rank.column();
        Self {
            suit,
            rank,
            face_up: false,
            sprite_index,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Pile {
    pub cards: Vec<Card>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StockAction {
    Drawn(usize),
    Recycled(usize),
    NoOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawMode {
    #[default]
    DrawOne,
    #[allow(dead_code)]
    DrawThree,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub stock: Pile,
    pub waste: Pile,
    pub foundations: [Pile; FOUNDATION_PILES],
    pub tableaus: [Pile; TABLEAU_PILES],
    pub draw_mode: DrawMode,
    pub score: i32,
    pub moves: u32,
    pub rng_seed: u64,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            stock: Pile::default(),
            waste: Pile::default(),
            foundations: Default::default(),
            tableaus: Default::default(),
            draw_mode: DrawMode::default(),
            score: 0,
            moves: 0,
            rng_seed: 0,
        }
    }

    pub fn deal_new_game(&mut self, draw_mode: DrawMode) -> Result<()> {
        let seed = random_seed()?;
        self.deal_with_seed(draw_mode, seed)
    }

    pub fn deal_again(&mut self) -> Result<()> {
        let seed = if self.rng_seed == 0 {
            random_seed()?
        } else {
            self.rng_seed
        };
        self.deal_with_seed(self.draw_mode, seed)
    }

    #[allow(dead_code)]
    pub fn deal_new_solvable(&mut self, draw_mode: DrawMode, max_attempts: usize) -> Result<usize> {
        let capped = max_attempts.min(120);
        let overall_deadline = std::time::Instant::now() + Duration::from_secs(10);
        for attempt in 1..=capped {
            self.deal_new_game(draw_mode)?;
            match self.is_solvable_result() {
                Some(true) => return Ok(attempt),
                Some(false) => continue,
                None => {
                    if std::time::Instant::now() >= overall_deadline {
                        break;
                    }
                }
            }
        }
        Err(anyhow!(
            "Failed to find solvable deal within {capped} attempts"
        ))
    }

    #[allow(dead_code)]
    pub fn is_solvable(&self) -> bool {
        matches!(self.is_solvable_result(), Some(true))
    }

    fn is_solvable_result(&self) -> Option<bool> {
        let deck = self.to_solver_deck()?;
        let draw = match self.draw_mode {
            DrawMode::DrawOne => 1,
            DrawMode::DrawThree => 3,
        };
        match solve_deck(&deck, draw, Duration::from_millis(SOLVER_TIME_BUDGET_MS)) {
            SolveResult::Winnable => Some(true),
            SolveResult::Unwinnable => Some(false),
            SolveResult::Timeout => None,
        }
    }
    fn to_solver_deck(&self) -> Option<[u8; 52]> {
        if self.rng_seed == 0 {
            return None;
        }

        let mut deck = create_standard_deck();
        shuffle_deck(&mut deck, self.rng_seed);

        let mut out = [0u8; 52];
        for (i, card) in deck.iter().enumerate() {
            out[i] = card.sprite_index;
        }
        Some(out)
    }

    fn deal_with_seed(&mut self, draw_mode: DrawMode, seed: u64) -> Result<()> {
        let mut deck = create_standard_deck();
        shuffle_deck(&mut deck, seed);

        self.draw_mode = draw_mode;
        self.score = 0;
        self.moves = 0;
        self.rng_seed = seed;
        self.waste.cards.clear();
        self.stock.cards.clear();
        for foundation in &mut self.foundations {
            foundation.cards.clear();
        }
        for tableau in &mut self.tableaus {
            tableau.cards.clear();
        }

        // Deal tableau: column i receives i+1 cards, last card face up.
        for column in 0..TABLEAU_PILES {
            let count = column + 1;
            let mut cards = Vec::with_capacity(count);
            for idx in 0..count {
                let mut card = deck
                    .pop()
                    .ok_or_else(|| anyhow!("Deck exhausted while dealing tableau"))?;
                card.face_up = idx == count - 1;
                cards.push(card);
            }
            self.tableaus[column].cards = cards;
        }

        // Remaining cards become the stock (all face down).
        for card in &mut deck {
            card.face_up = false;
        }
        self.stock.cards = deck;

        Ok(())
    }
    pub fn stock_click(&mut self) -> StockAction {
        if self.stock.cards.is_empty() {
            let recycled = self.recycle_stock();
            if recycled > 0 {
                StockAction::Recycled(recycled)
            } else {
                StockAction::NoOp
            }
        } else {
            let drawn = self.draw_from_stock();
            if drawn > 0 {
                StockAction::Drawn(drawn)
            } else {
                StockAction::NoOp
            }
        }
    }

    pub fn flip_tableau_top(&mut self, column: usize) -> bool {
        if let Some(pile) = self.tableaus.get_mut(column) {
            if let Some(card) = pile.cards.last_mut() {
                if !card.face_up {
                    card.face_up = true;
                    self.moves = self.moves.saturating_add(1);
                    self.score += 5;
                    return true;
                }
            }
        }
        false
    }

    pub fn move_waste_to_foundation(&mut self, foundation: usize) -> bool {
        if foundation >= FOUNDATION_PILES {
            return false;
        }
        let card = match self.waste.cards.pop() {
            Some(card) => card,
            None => return false,
        };
        if self.place_on_foundation(foundation, card) {
            true
        } else {
            self.waste.cards.push(card);
            false
        }
    }

    pub fn move_waste_to_tableau(&mut self, column: usize) -> bool {
        if column >= TABLEAU_PILES {
            return false;
        }
        let card = match self.waste.cards.last() {
            Some(card) => *card,
            None => return false,
        };
        if !can_place_on_tableau(card, self.tableaus[column].cards.last().copied()) {
            return false;
        }
        let card = self.waste.cards.pop().unwrap();
        self.tableaus[column].cards.push(card);
        self.moves = self.moves.saturating_add(1);
        true
    }

    pub fn move_tableau_to_foundation(&mut self, column: usize, foundation: usize) -> bool {
        if foundation >= FOUNDATION_PILES || column >= TABLEAU_PILES {
            return false;
        }
        let card = match self.tableaus[column].cards.last().copied() {
            Some(card) if card.face_up => card,
            _ => return false,
        };
        if !self.can_accept_foundation(foundation, card) {
            return false;
        }
        let card = self.tableaus[column].cards.pop().unwrap();
        if self.place_on_foundation(foundation, card) {
            self.reveal_tableau_top(column);
            true
        } else {
            false
        }
    }

    pub fn tableau_len(&self, column: usize) -> usize {
        self.tableaus.get(column).map_or(0, |pile| pile.cards.len())
    }

    pub fn tableau_card(&self, column: usize, index: usize) -> Option<&Card> {
        self.tableaus.get(column)?.cards.get(index)
    }

    pub fn extract_tableau_stack(&mut self, column: usize, index: usize) -> Option<Vec<Card>> {
        if column >= TABLEAU_PILES {
            return None;
        }
        let pile = self.tableaus.get_mut(column)?;
        if index >= pile.cards.len() {
            return None;
        }
        if !pile.cards[index].face_up {
            return None;
        }
        let mut stack = pile.cards.split_off(index);
        if !is_valid_tableau_run(&stack) {
            pile.cards.append(&mut stack);
            return None;
        }
        Some(stack)
    }

    pub fn cancel_tableau_stack(&mut self, column: usize, mut stack: Vec<Card>) {
        if column >= TABLEAU_PILES {
            return;
        }
        let pile = &mut self.tableaus[column];
        pile.cards.append(&mut stack);
    }

    pub fn can_accept_tableau_stack(&self, column: usize, stack: &[Card]) -> bool {
        if column >= TABLEAU_PILES || stack.is_empty() {
            return false;
        }
        if !is_valid_tableau_run(stack) {
            return false;
        }
        can_place_on_tableau(stack[0], self.tableaus[column].cards.last().copied())
    }

    pub fn place_tableau_stack(&mut self, column: usize, mut stack: Vec<Card>) -> bool {
        if !self.can_accept_tableau_stack(column, &stack) {
            return false;
        }
        let pile = &mut self.tableaus[column];
        pile.cards.append(&mut stack);
        self.moves = self.moves.saturating_add(1);
        true
    }

    pub fn reveal_tableau_top(&mut self, column: usize) {
        if column >= TABLEAU_PILES {
            return;
        }
        if let Some(card) = self.tableaus[column].cards.last_mut() {
            if !card.face_up {
                card.face_up = true;
                self.score += 5;
            }
        }
    }

    fn draw_from_stock(&mut self) -> usize {
        if self.stock.cards.is_empty() {
            return 0;
        }
        let draw_count = match self.draw_mode {
            DrawMode::DrawOne => 1,
            DrawMode::DrawThree => 3,
        }
        .min(self.stock.cards.len());
        let mut moved = 0;
        for _ in 0..draw_count {
            if let Some(mut card) = self.stock.cards.pop() {
                card.face_up = true;
                self.waste.cards.push(card);
                moved += 1;
            }
        }
        if moved > 0 {
            self.moves = self.moves.saturating_add(1);
        }
        moved
    }

    fn recycle_stock(&mut self) -> usize {
        if self.waste.cards.is_empty() {
            return 0;
        }
        let mut moved = 0;
        while let Some(mut card) = self.waste.cards.pop() {
            card.face_up = false;
            self.stock.cards.push(card);
            moved += 1;
        }
        if moved > 0 {
            self.moves = self.moves.saturating_add(1);
        }
        moved
    }

    #[allow(dead_code)]
    pub fn top_tableau_face_down(&self, column: usize) -> bool {
        self.tableaus
            .get(column)
            .and_then(|pile| pile.cards.last())
            .map(|card| !card.face_up)
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn waste_top(&self) -> Option<&Card> {
        self.waste.cards.last()
    }

    pub fn stock_count(&self) -> usize {
        self.stock.cards.len()
    }

    pub fn is_won(&self) -> bool {
        self.foundations.iter().all(|pile| pile.cards.len() == 13)
    }

    pub fn force_complete_foundations(&mut self) -> bool {
        if self.is_won() {
            return false;
        }
        let initial_foundation_cards: usize =
            self.foundations.iter().map(|pile| pile.cards.len()).sum();
        let mut collected = Vec::with_capacity(DECK_SIZE);
        let mut foundation_suits = [None; FOUNDATION_PILES];
        for (idx, foundation) in self.foundations.iter_mut().enumerate() {
            if let Some(card) = foundation.cards.last() {
                foundation_suits[idx] = Some(card.suit);
            }
            collected.append(&mut foundation.cards);
        }
        collected.append(&mut self.stock.cards);
        collected.append(&mut self.waste.cards);
        for tableau in &mut self.tableaus {
            collected.append(&mut tableau.cards);
        }
        if collected.is_empty() {
            return false;
        }
        let total_cards = collected.len();
        let mut per_suit: [Vec<Card>; FOUNDATION_PILES] = [
            Vec::with_capacity(13),
            Vec::with_capacity(13),
            Vec::with_capacity(13),
            Vec::with_capacity(13),
        ];
        for mut card in collected {
            card.face_up = true;
            let idx = card.suit.row() as usize;
            per_suit[idx].push(card);
        }
        for pile in &mut per_suit {
            pile.sort_by_key(|card| rank_value(card.rank));
        }
        let mut remaining_suits: Vec<Suit> = SUITS
            .iter()
            .copied()
            .filter(|suit| {
                !foundation_suits
                    .iter()
                    .flatten()
                    .any(|existing| existing == suit)
            })
            .collect();
        remaining_suits.reverse();
        for (idx, slot) in foundation_suits.iter_mut().enumerate() {
            let suit = slot
                .get_or_insert_with(|| remaining_suits.pop().unwrap_or(SUITS[idx % SUITS.len()]));
            let suit_index = suit.row() as usize;
            let cards = std::mem::take(&mut per_suit[suit_index]);
            self.foundations[idx].cards = cards;
        }
        let added_to_foundation = total_cards.saturating_sub(initial_foundation_cards);
        if added_to_foundation > 0 {
            self.moves = self.moves.saturating_add(added_to_foundation as u32);
            self.score += (added_to_foundation as i32) * 10;
        }
        for tableau in &mut self.tableaus {
            tableau.cards.clear();
        }
        self.stock.cards.clear();
        self.waste.cards.clear();
        true
    }
    pub fn can_accept_foundation(&self, foundation: usize, card: Card) -> bool {
        if foundation >= FOUNDATION_PILES {
            return false;
        }
        can_place_on_foundation(card, self.foundations[foundation].cards.last().copied())
    }

    pub fn place_on_foundation(&mut self, foundation: usize, card: Card) -> bool {
        if !self.can_accept_foundation(foundation, card) {
            return false;
        }
        self.foundations[foundation].cards.push(card);
        self.moves = self.moves.saturating_add(1);
        self.score += 10;
        true
    }

    pub fn move_waste_to_any_foundation(&mut self) -> bool {
        if let Some(card) = self.waste.cards.last().copied() {
            for idx in 0..FOUNDATION_PILES {
                if self.can_accept_foundation(idx, card) {
                    let card = self.waste.cards.pop().unwrap();
                    return self.place_on_foundation(idx, card);
                }
            }
        }
        false
    }

    pub fn move_tableau_top_to_any_foundation(&mut self, column: usize) -> bool {
        if column >= TABLEAU_PILES {
            return false;
        }
        let card = match self.tableaus[column].cards.last().copied() {
            Some(card) if card.face_up => card,
            _ => return false,
        };
        if let Some(idx) = (0..FOUNDATION_PILES).find(|&i| self.can_accept_foundation(i, card)) {
            let card = self.tableaus[column].cards.pop().unwrap();
            if self.place_on_foundation(idx, card) {
                self.reveal_tableau_top(column);
                return true;
            }
            return false;
        }
        false
    }

    pub fn waste_count(&self) -> usize {
        self.waste.cards.len()
    }

    pub fn tableau_column(&self, column: usize) -> Option<&[Card]> {
        self.tableaus.get(column).map(|pile| pile.cards.as_slice())
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

fn create_standard_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(DECK_SIZE);
    for suit in SUITS {
        for rank in RANKS {
            deck.push(Card::new(suit, rank));
        }
    }
    deck
}

fn shuffle_deck(deck: &mut [Card], seed: u64) {
    let mut rng = ShuffleRng::new(seed);
    for i in (1..deck.len()).rev() {
        let j = (rng.next_u32() as usize) % (i + 1);
        deck.swap(i, j);
    }
}

fn random_seed() -> Result<u64> {
    let mut bytes = [0u8; 8];
    fill_random(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn fill_random(bytes: &mut [u8]) -> Result<()> {
    let status = unsafe {
        BCryptGenRandom(
            BCRYPT_ALG_HANDLE::default(),
            bytes,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status == STATUS_SUCCESS {
        Ok(())
    } else {
        Err(anyhow!("BCryptGenRandom failed: 0x{:X}", status.0))
    }
}

fn is_valid_tableau_run(cards: &[Card]) -> bool {
    if cards.is_empty() {
        return false;
    }
    for card in cards {
        if !card.face_up {
            return false;
        }
    }
    for window in cards.windows(2) {
        let upper = window[0];
        let lower = window[1];
        if upper.suit.color() == lower.suit.color() {
            return false;
        }
        if rank_value(upper.rank) != rank_value(lower.rank) + 1 {
            return false;
        }
    }
    true
}

fn can_place_on_foundation(card: Card, top: Option<Card>) -> bool {
    match top {
        Some(top_card) => {
            card.suit == top_card.suit && rank_value(card.rank) == rank_value(top_card.rank) + 1
        }
        None => card.rank == Rank::Ace,
    }
}

fn can_place_on_tableau(card: Card, top: Option<Card>) -> bool {
    match top {
        Some(top_card) => {
            top_card.face_up
                && card.suit.color() != top_card.suit.color()
                && rank_value(card.rank) + 1 == rank_value(top_card.rank)
        }
        None => card.rank == Rank::King,
    }
}

fn rank_value(rank: Rank) -> u8 {
    rank as u8
}

struct ShuffleRng(u64);

impl ShuffleRng {
    fn new(seed: u64) -> Self {
        let seed = if seed == 0 { 0x4D44_5EED } else { seed };
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64* variant, deterministic per seed.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D)) >> 32) as u32
    }
}
