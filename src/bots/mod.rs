//! Contains all of the currently completed and experimental bots.

pub mod bot_iterative_parallel_mvv_lva;
pub mod basic;
pub mod lazy_smp;


use core::piece_move::BitMove;

pub struct BestMove {
    best_move: Option<BitMove>,
    score: i16,
}

impl BestMove {
    pub fn new(score: i16) -> Self {
        BestMove {
            best_move: None,
            score: score,
        }
    }

    pub fn negate(mut self) -> Self {
        self.score = self.score.wrapping_neg();
        self
    }
}
