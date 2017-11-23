//! A blazingly fast Chess Library.
//!
//! This package is seperated into two parts. Firstly, the board representation & associated functions (the current crate, `pleco`), And Secondly,
//! the AI implementations (`pleco_engine`).
//!
//! # Usage
//!
//! This crate is [on crates.io](https://crates.io/crates/pleco) and can be
//! used by adding `pleco` to the dependencies in your project's `Cargo.toml`.
//!
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", allow(inline_always))]
#![cfg_attr(feature="clippy", allow(unreadable_literal))]
#![cfg_attr(feature="clippy", allow(large_digit_groups))]
#![cfg_attr(feature="clippy", allow(cast_lossless))]
#![cfg_attr(feature="clippy", allow(doc_markdown))]
#![cfg_attr(feature="clippy", allow(inconsistent_digit_grouping))]

#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(test, allow(dead_code))]

#![feature(integer_atomics)]
#![feature(test)]
#![allow(dead_code)]
#![feature(integer_atomics)]
#![feature(unique)]
#![feature(allocator_api)]


#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate lazy_static;

extern crate test;
extern crate rayon;
extern crate num_cpus;
extern crate rand;

pub mod core;
pub mod board;
pub mod tools;

pub mod engine;

pub mod bots;
pub mod bot_prelude;

#[doc(no_inline)]
pub use board::Board;
#[doc(no_inline)]
pub use core::piece_move::BitMove;
#[doc(no_inline)]
pub use core::sq::SQ;
#[doc(no_inline)]
pub use core::bitboard::BitBoard;
#[doc(no_inline)]
pub use core::{Player,Piece,Rank,File};


