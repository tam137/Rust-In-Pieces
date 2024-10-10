mod fen_service;          // Importiere den FEN-Service
mod move_gen_service;     // Importiere den Zug-Generierungsservice
mod notation_util;        // Importiere den Notations-Hilfsservice
mod model;                // Importiere das Modul für alle Modelle

use crate::fen_service::FenServiceImpl;
use crate::move_gen_service::MoveGenService; // Importiere spezifische Strukturen aus model.rs

fn main() {
    // Instantiere die benötigten Services
    let fen_service = FenServiceImpl;
    let move_gen_service = MoveGenService;

    // Beispiel FEN-String (Standard Schachbrett-Setup)
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    // Setze das Board mit dem FEN-String
    let mut board = fen_service.set_fen(fen);

    // Generiere gültige Züge
    let valid_moves = move_gen_service.generate_valid_moves_list(&mut board);

    // Gebe die Züge aus
    println!("Valid Moves: {:?}", valid_moves);

    // Beende das Programm
    println!("Zug-Generator abgeschlossen.");
}
