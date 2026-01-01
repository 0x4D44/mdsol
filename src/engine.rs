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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deal_new_game() {
        let mut game = GameState::new();
        game.deal_new_game(DrawMode::DrawOne).unwrap();

        // Check stock size
        // 52 cards total.
        // Tableau cards: 1+2+3+4+5+6+7 = 28 cards.
        // Remaining in stock: 52 - 28 = 24.
        assert_eq!(game.stock.cards.len(), 24);
        assert_eq!(game.waste.cards.len(), 0);

        // Check tableau structure
        for i in 0..7 {
            assert_eq!(game.tableaus[i].cards.len(), i + 1);
            // Verify face up/down
            for (j, card) in game.tableaus[i].cards.iter().enumerate() {
                if j == i {
                    assert!(card.face_up, "Top card of tableau {} should be face up", i);
                } else {
                    assert!(!card.face_up, "Card {} of tableau {} should be face down", j, i);
                }
            }
        }
    }

    #[test]
    fn test_stock_mechanics_draw_one() {
        let mut game = GameState::new();
        game.deal_new_game(DrawMode::DrawOne).unwrap();
        let initial_stock = game.stock.cards.len();

        // Draw 1
        match game.stock_click() {
            StockAction::Drawn(n) => assert_eq!(n, 1),
            _ => panic!("Expected Drawn(1)"),
        }
        assert_eq!(game.stock.cards.len(), initial_stock - 1);
        assert_eq!(game.waste.cards.len(), 1);
        assert!(game.waste.cards[0].face_up);

        // Draw all remaining
        for _ in 0..(initial_stock - 1) {
            game.stock_click();
        }
        assert_eq!(game.stock.cards.len(), 0);
        assert_eq!(game.waste.cards.len(), initial_stock);

        // Recycle
        match game.stock_click() {
            StockAction::Recycled(n) => assert_eq!(n, initial_stock),
            _ => panic!("Expected Recycled({})", initial_stock),
        }
        assert_eq!(game.stock.cards.len(), initial_stock);
        assert_eq!(game.waste.cards.len(), 0);
        // Verify they are face down again
        assert!(game.stock.cards.iter().all(|c| !c.face_up));
    }

    #[test]
    fn test_stock_mechanics_draw_three() {
        let mut game = GameState::new();
        game.deal_new_game(DrawMode::DrawThree).unwrap();
        // 24 cards in stock.
        
        // Draw 3
        match game.stock_click() {
            StockAction::Drawn(n) => assert_eq!(n, 3),
            _ => panic!("Expected Drawn(3)"),
        }
        assert_eq!(game.stock.cards.len(), 21);
        assert_eq!(game.waste.cards.len(), 3);

        // Draw until < 3 left. 
        // 21 / 3 = 7 more draws.
        for _ in 0..7 {
            game.stock_click();
        }
        assert_eq!(game.stock.cards.len(), 0);
        
        // Recycle
        game.stock_click();
        assert_eq!(game.stock.cards.len(), 24);
    }

    #[test]
    fn test_foundation_rules() {
        let mut game = GameState::new();
        // Spades Ace
        let ace_spades = Card::new(Suit::Spades, Rank::Ace);
        // Spades Two
        let two_spades = Card::new(Suit::Spades, Rank::Two);
        // Hearts Ace
        let ace_hearts = Card::new(Suit::Hearts, Rank::Ace);

        // Foundation 0 is empty. Ace Spades should be valid.
        assert!(game.can_accept_foundation(0, ace_spades));

        // Two Spades invalid on empty
        assert!(!game.can_accept_foundation(0, two_spades));

        // Place Ace Spades
        game.place_on_foundation(0, ace_spades);

        // Now Two Spades valid
        assert!(game.can_accept_foundation(0, two_spades));

        // Ace Hearts invalid on Spades
        assert!(!game.can_accept_foundation(0, ace_hearts));
    }

    #[test]
    fn test_tableau_rules() {
        let mut game = GameState::new();

        // Clear a tableau for testing empty column rules
        game.tableaus[0].cards.clear();

        let mut king_spades = Card::new(Suit::Spades, Rank::King);
        king_spades.face_up = true;
        let mut queen_hearts = Card::new(Suit::Hearts, Rank::Queen);
        queen_hearts.face_up = true;
        let mut queen_spades = Card::new(Suit::Spades, Rank::Queen); // Same color as King Spades
        queen_spades.face_up = true;
        let mut jack_spades = Card::new(Suit::Spades, Rank::Jack);
        jack_spades.face_up = true;

        // Only King on empty
        assert!(game.can_accept_tableau_stack(0, &[king_spades]));
        assert!(!game.can_accept_tableau_stack(0, &[queen_hearts]));

        // Place King
        game.tableaus[0].cards.push(king_spades);
        
        // Queen Hearts (Red) on King Spades (Black) -> Valid
        assert!(game.can_accept_tableau_stack(0, &[queen_hearts]));

        // Queen Spades (Black) on King Spades (Black) -> Invalid (same color)
        assert!(!game.can_accept_tableau_stack(0, &[queen_spades]));

        // Jack Spades on King Spades -> Invalid (skip rank)
        assert!(!game.can_accept_tableau_stack(0, &[jack_spades]));
    }

    #[test]
    fn test_stack_moves() {
        // Setup: Column 0 has King Spades. Column 1 has nothing.
        // We want to verify we can move a stack of (Queen Hearts -> Jack Spades) onto King Spades.
        let mut game = GameState::new();
        game.tableaus[0].cards.clear();
        game.tableaus[0].cards.push(Card {
            suit: Suit::Spades,
            rank: Rank::King,
            face_up: true,
            sprite_index: 0,
        });

        let stack = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Queen,
                face_up: true,
                sprite_index: 0,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Jack,
                face_up: true,
                sprite_index: 0,
            },
        ];

        assert!(game.can_accept_tableau_stack(0, &stack));

        // Invalid stack (face down card inside)
        let mut bad_stack = stack.clone();
        bad_stack[0].face_up = false;
        assert!(!game.can_accept_tableau_stack(0, &bad_stack));

        // Invalid stack (color sequence broken)
        let bad_seq = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Queen,
                face_up: true,
                sprite_index: 0,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Jack,
                face_up: true,
                sprite_index: 0,
            }, // Red on Red
        ];
        assert!(!game.can_accept_tableau_stack(0, &bad_seq));
    }

    #[test]
    fn test_auto_complete() {
        let mut game = GameState::new();
        game.deal_new_game(DrawMode::DrawOne).unwrap();

        // Cheat: force completion
        assert!(game.force_complete_foundations());
        assert!(game.is_won());
        assert_eq!(game.foundations[0].cards.len(), 13);
        assert_eq!(game.foundations[1].cards.len(), 13);
        assert_eq!(game.foundations[2].cards.len(), 13);
        assert_eq!(game.foundations[3].cards.len(), 13);
        assert!(game.stock.cards.is_empty());
        assert!(game.waste.cards.is_empty());
    }

    #[test]
    fn test_deal_again() {
        let mut game = GameState::new();
        game.deal_new_game(DrawMode::DrawOne).unwrap();
        let seed = game.rng_seed;
        let card0 = game.stock.cards[0].clone();

        game.deal_again().unwrap();
        assert_eq!(game.rng_seed, seed);
        // Deck should be identical.
        assert_eq!(game.stock.cards[0].rank, card0.rank);
        assert_eq!(game.stock.cards[0].suit, card0.suit);
    }

    #[test]
    fn test_gameplay_flow() {
        let mut game = GameState::new();
        game.deal_new_game(DrawMode::DrawOne).unwrap();

        // Clear board to force specific state
        for t in &mut game.tableaus {
            t.cards.clear();
        }
        for f in &mut game.foundations {
            f.cards.clear();
        }
        game.waste.cards.clear();

        // Setup specific move: Waste -> Tableau
        let king_spades = Card {
            suit: Suit::Spades,
            rank: Rank::King,
            face_up: true,
            sprite_index: 0,
        };
        game.waste.cards.push(king_spades);

        // Move waste to tableau 0 (empty)
        assert!(game.move_waste_to_tableau(0));
        assert_eq!(game.tableaus[0].cards.len(), 1);
        assert_eq!(game.waste.cards.len(), 0);

        // Move tableau -> foundation
        // Put Ace Spades on Tableau 1 (empty).
        let ace_spades = Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
            face_up: true,
            sprite_index: 0,
        };
        game.tableaus[1].cards.push(ace_spades);

        assert!(game.move_tableau_to_foundation(1, 0)); // Column 1 to Foundation 0
        assert_eq!(game.foundations[0].cards.len(), 1); // Ace Spades
        assert_eq!(game.tableaus[1].cards.len(), 0);
    }

    #[test]
    fn test_convenience_moves() {
        let mut game = GameState::new();
        game.deal_new_game(DrawMode::DrawOne).unwrap();

        // Setup Waste -> Any Foundation
        game.waste.cards.clear();
        game.waste.cards.push(Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
            face_up: true,
            sprite_index: 0,
        });

        assert!(game.move_waste_to_any_foundation());
        // Foundation 0 is empty, so it takes it.
        assert_eq!(game.foundations[0].cards.len(), 1);
        assert_eq!(game.foundations[0].cards[0].suit, Suit::Spades);

        // Setup Tableau -> Any Foundation
        game.tableaus[0].cards.clear();
        game.tableaus[0].cards.push(Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
            face_up: true,
            sprite_index: 0,
        });

        assert!(game.move_tableau_top_to_any_foundation(0));
        // Foundation 0 has Ace Spades. Ace Hearts can't go there.
        // Foundation 1 is empty. It should go there.
        assert_eq!(game.foundations[1].cards.len(), 1);
        assert_eq!(game.foundations[1].cards[0].suit, Suit::Hearts);

        // Test Flip
        // Put a face down card in tableau 0.
        game.tableaus[0].cards.push(Card {
            suit: Suit::Clubs,
            rank: Rank::Two,
            face_up: false,
            sprite_index: 0,
        });
        assert!(!game.tableaus[0].cards[0].face_up);

        assert!(game.flip_tableau_top(0));
        assert!(game.tableaus[0].cards[0].face_up);

        // Flipping again should do nothing
        assert!(!game.flip_tableau_top(0));
    }
}
