//! Suprah chess engine library: every engine module, shared by the UCI engine binary
//! (`src/main.rs`) and the training-data generator (`src/bin/rip-datagen.rs`).

#![allow(clippy::too_many_arguments)]

pub mod book;
pub mod config;
pub mod datagen;
pub mod eval_service;
pub mod fen_service;
pub mod game_handler;
pub mod magic;
pub mod model;
pub mod move_gen_service;
pub mod nnue_prof;
pub mod nnue_service;
pub mod notation_util;
pub mod pawn_hash;
pub mod polyglot;
pub mod pst;
pub mod search_diag;
pub mod search_service;
pub mod service;
pub mod stdout_wrapper;
pub mod threads;
pub mod time_check;
pub mod uci_parser_service;
pub mod zobrist;
