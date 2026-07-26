import re

# Exact map of positions present in book.rs
# FEN strings -> list of allowed moves in book.rs
book_rs_fen_moves = {
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1": ["e2e4", "d2d4", "g1f3", "c2c4", "b2b3", "b2b4", "f2f4", "g2g4", "c2c3", "a2a3"],
    "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1": ["e7e5", "c7c5", "e7e6", "c7c6", "d7d6", "d7d5", "g7g6", "b8c6"],
    "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1": ["e7e6", "d7d5", "d7d6", "g8f6"],
    "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 0 1": ["e7e6", "d7d5", "d7d6", "g8f6", "c7c5"],
    "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq c3 0 1": ["e7e6", "d7d5", "d7d6", "g8f6", "b8c6", "c7c5"],
    "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2": ["e4d5", "d2d3"],
    "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2": ["g1f3", "f1c4", "d2d4", "d2d3", "b1c3"],
    "rnbqkbnr/ppp1pppp/3p4/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2": ["d2d4", "d2d3", "g1f3", "b1c3", "f1c4", "c2c4", "c2c3"],
    "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2": ["e4e5", "d2d3", "b1c3", "d1f3"],
    "rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2": ["d2d4", "d2d3", "g1f3", "b1c3", "f1c4", "c2c4", "c2c3", "e4e5"],
    "rnbqkbnr/pp1ppppp/2p5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2": ["d2d4", "g1f3", "b1c3", "c2c3", "d2d3"],
    "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2": ["g1f3", "b1c3", "c2c3", "d2d4", "f2f4", "d2d3"],
    "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2": ["b8c6", "g8f6", "d7d6"],
    "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3": ["f1b5", "f1c4", "b1c3", "d2d4"],
    "rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2": ["d7d6", "e7e6", "b8c6", "g7g6"],
    "rnbqkbnr/pppp1ppp/4p3/8/3PP3/8/PPP2PPP/RNBQKBNR b KQkq d3 0 2": ["d7d5"],
    "rnbqkbnr/pp1ppppp/2p5/8/3PP3/8/PPP2PPP/RNBQKBNR b KQkq d3 0 2": ["d7d5"],
    "rnbqkbnr/ppp1pppp/8/3p4/2PP4/8/PP2PPPP/RNBQKBNR b KQkq c3 0 2": ["e7e6", "c7c6", "d5c4"],
    "rnbqkbnr/ppp1pppp/3p4/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2": ["c2c4", "e2e4", "e2e3", "g1f3"],
    "rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq d6 0 2": ["c2c4", "c2c3", "e2e3", "c1f4", "g1f3", "b1c3", "e2e4"],
    "rnbqkbnr/pppp1ppp/4p3/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2": ["c2c4", "c2c3", "e2e3", "c1f4", "g1f3", "b1c3", "e2e4"],
    "rnbqkb1r/pppppppp/5n2/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2": ["c2c4", "c1f4", "c1g5", "g1f3", "e2e3"],
    "rnbqkbnr/ppp2ppp/4p3/3p4/3PP3/2N5/PPP2PPP/R1BQKBNR b KQkq - 0 3": ["g8f6", "f8b4", "d5e4"],
}

print("=== CHECKING GAMES IN PGN FOR MOVES ABSENT FROM BOOK.RS ===")

with open("/home/mattmagie/mattmagie/all.pgn", "r", encoding="utf-8", errors="ignore") as f:
    content = f.read()

games = content.split("[Event ")

poly_engines = ["0.18.1", "0.19.0", "0.19.1"]

non_book_rs_moves = []

for g in games:
    if not g.strip(): continue
    w_match = re.search(r'\[White "([^"]+)"\]', g)
    b_match = re.search(r'\[Black "([^"]+)"\]', g)
    if not w_match or not b_match: continue
    w_eng, b_eng = w_match.group(1), b_match.group(1)

    if not any(v in w_eng for v in poly_engines) and not any(v in b_eng for v in poly_engines):
        continue

    lines = g.strip().split("\n")
    moves_text = " ".join([l for l in lines if not l.startswith("[") and l.strip()])
    
    # Check 3rd move White in French Defense: 1. e4 e6 2. d4 d5 3. ?
    if "1. e2e4 e7e6 2. d2d4 d7d5" in moves_text:
        m3 = re.search(r"3\.\s*(\S+)", moves_text)
        if m3:
            move3 = m3.group(1)
            # book.rs only has 3. Nc3 (Nb1c3)
            if move3 not in ["Nb1c3", "Nc3", "b1c3"]:
                non_book_rs_moves.append(("French 3rd Move (not Nc3)", move3, w_eng, b_eng, moves_text[:120]))

    # Check Sicilian 2nd/3rd move responses: e.g. 1. e4 c5 2. Nf3 Nc6 3. Bb5 (Rossolimo) or 3. d4 cxd4 4. Nxd4 e5 (Kalashnikov) / g6 (Accelerated Dragon)
    if "1. e2e4 c7c5 2. Ng1f3 b8c6 3. f1b5" in moves_text or "1. e2e4 c7c5 2. Nf3 Nc6 3. Bb5" in moves_text:
        non_book_rs_moves.append(("Sicilian Rossolimo (3. Bb5)", "f1b5", w_eng, b_eng, moves_text[:120]))

    # Check Ruy Lopez 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 / Nxe4
    if "3. f1b5 a7a6 4. b5a4" in moves_text or "3. Bb5 a6 4. Ba4" in moves_text:
        non_book_rs_moves.append(("Ruy Lopez Ba4 (not in book.rs)", "b5a4", w_eng, b_eng, moves_text[:120]))

    # Check Petrov 3. d4 (Steinitz)
    if "1. e2e4 e7e5 2. Ng1f3 Ng8f6 3. d2d4" in moves_text:
        non_book_rs_moves.append(("Petrov Steinitz (3. d4)", "d2d4", w_eng, b_eng, moves_text[:120]))

print(f"Total non-book.rs external book moves detected: {len(non_book_rs_moves)}")
for cat, mv, w, b, snippet in non_book_rs_moves[:15]:
    print(f"  [{cat}] Move: {mv} | White: {w} vs Black: {b}")
    print(f"     PGN: {snippet}")
