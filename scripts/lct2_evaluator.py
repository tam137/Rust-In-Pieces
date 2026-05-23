#!/usr/bin/env python3
import subprocess
import time
import sys
import threading
import queue
import argparse
import os

# ANSI escape codes for premium styled console output
GREEN = "\033[0;32m"
RED = "\033[0;31m"
YELLOW = "\033[0;33m"
CYAN = "\033[0;36m"
MAGENTA = "\033[0;35m"
BLUE = "\033[0;34m"
BOLD = "\033[1m"
UNDERLINE = "\033[4m"
NC = "\033[0m"

# The LCT II database of 35 positions (14 Positional, 12 Tactical, 9 Endgame)
LCT2_POSITIONS = [
    # ---- 14 POSITIONAL POSITIONS ----
    {
        "id": "LCTII.POS.01",
        "fen": "r3kb1r/3n1pp1/p6p/2pPp2q/Pp2N3/3B2PP/1PQ2P2/R3K2R w KQkq - 0 1",
        "bm": "d5d6",
        "cat": "Positional",
        "desc": "Chernin - Miles, Tunis 1985"
    },
    {
        "id": "LCTII.POS.02",
        "fen": "1k1r3r/pp2qpp1/3b1n1p/3pNQ2/2pP1P2/2N1P3/PP4PP/1K1RR3 b - - 0 1",
        "bm": "d6b4",
        "cat": "Positional",
        "desc": "Lilienthal - Botvinnik, Moskau 1945"
    },
    {
        "id": "LCTII.POS.03",
        "fen": "r6k/pp4p1/2p1b3/3pP3/7q/P2B3r/1PP2Q1P/2K1R1R1 w - - 0 1",
        "bm": "f2c5",
        "cat": "Positional",
        "desc": "Boissel - Boulard, corr. 1994"
    },
    {
        "id": "LCTII.POS.04",
        "fen": "1nr5/2rbkppp/p3p3/Np6/2PRPP2/8/PKP1B1PP/3R4 b - - 0 1",
        "bm": "e6e5",
        "cat": "Positional",
        "desc": "Kaplan - Kopec, USA 1975"
    },
    {
        "id": "LCTII.POS.05",
        "fen": "2r2rk1/1p1bq3/p3p2p/3pPpp1/1P1Q4/P7/2P2PPP/2R1RBK1 b - - 0 1",
        "bm": "d7b5",
        "cat": "Positional",
        "desc": "Estrin - Pytel, Albena 1973"
    },
    {
        "id": "LCTII.POS.06",
        "fen": "3r1bk1/p4ppp/Qp2p3/8/1P1B4/Pq2P1P1/2r2P1P/R3R1K1 b - - 0 1",
        "bm": "e6e5",
        "cat": "Positional",
        "desc": "Nimzowitsch - Marshall 1927"
    },
    {
        "id": "LCTII.POS.07",
        "fen": "r1b2r1k/pp2q1pp/2p2p2/2p1n2N/4P3/1PNP2QP/1PP2RP1/5RK1 w - - 0 1",
        "bm": "c3d1",
        "cat": "Positional",
        "desc": "Alehine - Nimzowitsch, Semmering 1926"
    },
    {
        "id": "LCTII.POS.08",
        "fen": "r2qrnk1/pp3ppb/3b1n1p/1Pp1p3/2P1P2N/P5P1/1B1NQPBP/R4RK1 w - - 0 1",
        "bm": "g2h3",
        "cat": "Positional",
        "desc": "Unzicker - Fischer, Varna 1962"
    },
    {
        "id": "LCTII.POS.09",
        "fen": "5nk1/Q4bpp/5p2/8/P1n1PN2/q4P2/6PP/1R4K1 w - - 0 1",
        "bm": "a7d4",
        "cat": "Positional",
        "desc": "Boissel - Del Gobbo, corr. 1994"
    },
    {
        "id": "LCTII.POS.10",
        "fen": "r3k2r/3bbp1p/p1nppp2/5P2/1p1NP3/5NP1/PPPK3P/3R1B1R b kq - 0 1",
        "bm": "e7f8",
        "cat": "Positional",
        "desc": "A.Sokolov - Salov, Leningrad 1987"
    },
    {
        "id": "LCTII.POS.11",
        "fen": "bn6/1q4n1/1p1p1kp1/2pPp1pp/1PP1P1P1/3N1P1P/4B1K1/2Q2N2 w - - 0 1",
        "bm": "h3h4",
        "cat": "Positional",
        "desc": "Capablanca - Ragozin, Moskau 1935"
    },
    {
        "id": "LCTII.POS.12",
        "fen": "3r2k1/pp2npp1/2rqp2p/8/3PQ3/1BR3P1/PP3P1P/3R2K1 b - - 0 1",
        "bm": "c6b6",
        "cat": "Positional",
        "desc": "Zuckerman - Evans, USA 1967"
    },
    {
        "id": "LCTII.POS.13",
        "fen": "1r2r1k1/4ppbp/B5p1/3P4/pp1qPB2/2n2Q1P/P4PP1/4RRK1 b - - 0 1",
        "bm": "c3a2",
        "cat": "Positional",
        "desc": "Karpov - Kasparov, Moskau 1985"
    },
    {
        "id": "LCTII.POS.14",
        "fen": "r2qkb1r/1b3ppp/p3pn2/1p6/1n1P4/1BN2N2/PP2QPPP/R1BR2K1 w kq - 0 1",
        "bm": "d4d5",
        "cat": "Positional",
        "desc": "Polugaevsky - Nezhmetdinov, Sochi 1958"
    },

    # ---- 12 TACTICAL POSITIONS ----
    {
        "id": "LCTII.TAC.01",
        "fen": "1r4k1/1q2bp2/3p2p1/2pP4/p1N4R/2P2QP1/1P3PK1/8 w - - 0 1",
        "bm": "c4d6",
        "cat": "Tactical",
        "desc": "Zubarev - Geller, USSR 1950"
    },
    {
        "id": "LCTII.TAC.02",
        "fen": "rn3rk1/pbppq1pp/1p2pb2/4N2Q/3PN3/3B2PP/PPP2PPP/R3K2R w KQ - 0 1",
        "bm": "h5h7",
        "cat": "Tactical",
        "desc": "Keres - Eliskases, Noordwijk 1938"
    },
    {
        "id": "LCTII.TAC.03",
        "fen": "4r1k1/3b1p2/5qp1/1BPpn2p/7n/r3P1N1/2Q1RPPP/1R3NK1 b - - 0 1",
        "bm": "f6f3",
        "cat": "Tactical",
        "desc": "Drimer - Rellstab, corr. 1968"
    },
    {
        "id": "LCTII.TAC.04",
        "fen": "2k2b1r/1pq3p1/2p1pp2/p1n1PnNp/2P2B2/2N4P/PP2QPP1/3R2K1 w - - 0 1",
        "bm": "e5f6",
        "cat": "Tactical",
        "desc": "Hort - Wade, Pajulahti 1974"
    },
    {
        "id": "LCTII.TAC.05",
        "fen": "2r2r2/3qbpkp/p3n1p1/2ppP3/6Q1/1P1B3R/PBP3PP/5R1K w - - 0 1",
        "bm": "h3h7",
        "cat": "Tactical",
        "desc": "Fischer - Myagmarsuren, Sousse 1967"
    },
    {
        "id": "LCTII.TAC.06",
        "fen": "2r1k2r/2pn1pp1/1p3n1p/p3PP2/4q2B/P1P5/2Q1N1PP/R4RK1 w q - 0 1",
        "bm": "e5f6",
        "cat": "Tactical",
        "desc": "R.Byrne - Fischer, New York 1963"
    },
    {
        "id": "LCTII.TAC.07",
        "fen": "2rr2k1/1b3ppp/pb2p3/1p2P3/1P2BPnq/P1N3P1/1B2Q2P/R4R1K b - - 0 1",
        "bm": "c8c3",
        "cat": "Tactical",
        "desc": "Wojtkiewicz - Kasparov, Simultan 1993"
    },
    {
        "id": "LCTII.TAC.08",
        "fen": "2b1r1k1/r4ppp/p7/2pNP3/4Q3/q6P/2P2PP1/3RR1K1 w - - 0 1",
        "bm": "d5f6",
        "cat": "Tactical",
        "desc": "Nei - Bronstein, Moskau 1963"
    },
    {
        "id": "LCTII.TAC.09",
        "fen": "6k1/5p2/3P2p1/7n/3QPP2/7q/r2N3P/6RK b - - 0 1",
        "bm": "a2d2",
        "cat": "Tactical",
        "desc": "Stein - Birbrager, USSR 1966"
    },
    {
        "id": "LCTII.TAC.10",
        "fen": "rq2rbk1/6p1/p2p2Pp/1p1Rn3/4PB2/6Q1/PPP1B3/2K3R1 w - - 0 1",
        "bm": "f4h6",
        "cat": "Tactical",
        "desc": "Fischer - Gadia, Simultan 1965"
    },
    {
        "id": "LCTII.TAC.11",
        "fen": "rnbq2k1/p1r2p1p/1p1p1Pp1/1BpPn1N1/P7/2P5/6PP/R1B1QRK1 w - - 0 1",
        "bm": "g5h7",
        "cat": "Tactical",
        "desc": "Nezhmetdinov - Tal, Baku 1961"
    },
    {
        "id": "LCTII.TAC.12",
        "fen": "r2qrb1k/1p1b2p1/p2ppn1p/8/3NP3/1BN5/PPP3QP/1K3RR1 w - - 0 1",
        "bm": "e4e5",
        "cat": "Tactical",
        "desc": "Vaganyan - Kupreichik, USSR 1980"
    },

    # ---- 9 ENDGAME POSITIONS ----
    {
        "id": "LCTII.END.01",
        "fen": "8/1p3pp1/7p/5P1P/2k3P1/8/2K2P2/8 w - - 0 1",
        "bm": "f5f6",
        "cat": "Endgame",
        "desc": "Pawn Endgame Study"
    },
    {
        "id": "LCTII.END.02",
        "fen": "8/pp2r1k1/2p1p3/3pP2p/1P1P1P1P/P5KR/8/8 w - - 0 1",
        "bm": "f4f5",
        "cat": "Endgame",
        "desc": "Rook Endgame Study"
    },
    {
        "id": "LCTII.END.03",
        "fen": "8/3p4/p1bk3p/Pp6/1Kp1PpPp/2P2P1P/2P5/5B2 b - - 0 1",
        "bm": "c6e4",
        "cat": "Endgame",
        "desc": "Bishop Endgame Study"
    },
    {
        "id": "LCTII.END.04",
        "fen": "5k2/7R/4P2p/5K2/p1r2P1p/8/8/8 b - - 0 1",
        "bm": "h4h3",
        "cat": "Endgame",
        "desc": "Rook and Pawn Study"
    },
    {
        "id": "LCTII.END.05",
        "fen": "6k1/6p1/7p/P1N5/1r3p2/7P/1b3PP1/3bR1K1 w - - 0 1",
        "bm": "a5a6",
        "cat": "Endgame",
        "desc": "Endgame Combination Study"
    },
    {
        "id": "LCTII.END.06",
        "fen": "8/3b4/5k2/2pPnp2/1pP4N/pP1B2P1/P3K3/8 b - - 0 1",
        "bm": "f5f4",
        "cat": "Endgame",
        "desc": "Knight and Bishop Study"
    },
    {
        "id": "LCTII.END.07",
        "fen": "6k1/4pp1p/3p2p1/P1pPb3/R7/1r2P1PP/3B1P2/6K1 w - - 0 1",
        "bm": "d2b4",
        "cat": "Endgame",
        "desc": "Endgame Rook Slide Study"
    },
    {
        "id": "LCTII.END.08",
        "fen": "2k5/p7/Pp1p1b2/1P1P1p2/2P2P1p/3K3P/5B2/8 w - - 0 1",
        "bm": "c4c5",
        "cat": "Endgame",
        "desc": "Positional Pawn Breakthrough Study"
    },
    {
        "id": "LCTII.END.09",
        "fen": "8/5Bp1/4P3/6pP/1b1k1P2/5K2/8/8 w - - 0 1",
        "bm": "f3g4",
        "cat": "Endgame",
        "desc": "King and Bishop Endgame Study"
    }
]

def reader_thread(stream, out_queue):
    """Background thread to read stdout of the engine non-blockingly."""
    for line in iter(stream.readline, ''):
        out_queue.put(line)
    stream.close()

def run_uci_test(binary_path, positions, timeout):
    """Runs the LCT II test suite and evaluates the engine."""
    
    print(f"\n{CYAN}================================================================{NC}")
    print(f"{CYAN}{BOLD}              LCT II (LOUGUET CHESS TEST II) EVALUATOR          {NC}")
    print(f"{CYAN}================================================================{NC}")
    print(f"Engine Binary: {BOLD}{binary_path}{NC}")
    print(f"Timeout limit: {BOLD}{timeout} seconds per position{NC}")
    print(f"Total positions: {BOLD}{len(positions)}{NC}")
    print(f"{CYAN}----------------------------------------------------------------{NC}\n")

    total_points = 0
    solved_count = 0
    cat_stats = {
        "Positional": {"solved": 0, "total": 14, "points": 0},
        "Tactical": {"solved": 0, "total": 12, "points": 0},
        "Endgame": {"solved": 0, "total": 9, "points": 0}
    }

    # Print table header
    print(f"{BOLD}{'ID':<13} | {'Category':<10} | {'Correct':<7} | {'Engine':<7} | {'Solved?':<7} | {'Time (s)':<8} | {'Points':<6}{NC}")
    print(f"--------------------------------------------------------------------------------")

    for p in positions:
        pos_id = p["id"]
        fen = p["fen"]
        correct_move = p["bm"]
        category = p["cat"]
        
        # Start subprocess
        proc = subprocess.Popen(
            [binary_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1
        )

        out_queue = queue.Queue()
        t = threading.Thread(target=reader_thread, args=(proc.stdout, out_queue))
        t.daemon = True
        t.start()

        # Initialize UCI connection
        proc.stdin.write("uci\n")
        proc.stdin.write("isready\n")
        proc.stdin.flush()
        
        # Read until readyok
        ready = False
        while not ready:
            try:
                line = out_queue.get(timeout=2)
                if "readyok" in line:
                    ready = True
            except queue.Empty:
                break

        if not ready:
            print(f"{RED}{pos_id:<13} | {category:<10} | {correct_move:<7} | {'ERROR':<7} | {'NO':<7} | {'-':<8} | {'0':<6}{NC}")
            proc.terminate()
            continue

        # Send position and start search
        proc.stdin.write(f"position fen {fen}\n")
        proc.stdin.write("go infinite\n")
        proc.stdin.flush()
        
        start_time = time.time()
        first_solve_time = None
        current_best = None
        
        # Monitor search
        bestmove = None
        engine_stopped_early = False
        while True:
            elapsed = time.time() - start_time
            if elapsed > timeout:
                break

            # Read all available stdout lines
            lines_read = []
            while True:
                try:
                    line = out_queue.get_nowait()
                    lines_read.append(line.strip())
                except queue.Empty:
                    break

            for line in lines_read:
                if line.startswith("bestmove"):
                    try:
                        bestmove = line.split()[1]
                    except IndexError:
                        pass
                    engine_stopped_early = True
                    break

                if line.startswith("info ") and "pv " in line:
                    parts = line.split("pv ")
                    if len(parts) > 1:
                        pv_moves = parts[1].split()
                        if pv_moves:
                            best_pv_move = pv_moves[0]
                            if best_pv_move == correct_move:
                                if current_best != correct_move:
                                    current_best = correct_move
                                    first_solve_time = elapsed
                            else:
                                current_best = best_pv_move
                                first_solve_time = None

            if engine_stopped_early:
                break

            time.sleep(0.01)

        if not engine_stopped_early:
            # Stop search and fetch bestmove
            proc.stdin.write("stop\n")
            proc.stdin.flush()
            
            while True:
                try:
                    line = out_queue.get(timeout=2)
                    if line.startswith("bestmove"):
                        bestmove = line.split()[1]
                        break
                except (queue.Empty, IndexError):
                    break

        try:
            proc.stdin.write("quit\n")
            proc.stdin.flush()
        except Exception:
            pass
        proc.terminate()

        # Evaluate correctness
        solved = False
        solve_time = 9999.0
        
        if bestmove == correct_move:
            solved = True
            solved_count += 1
            cat_stats[category]["solved"] += 1
            if first_solve_time is not None:
                solve_time = first_solve_time
            else:
                solve_time = time.time() - start_time
        
        # Calculate points
        points = 0
        if solved:
            if solve_time <= 9.0:
                points = 30
            elif solve_time <= 29.0:
                points = 25
            elif solve_time <= 89.0:
                points = 20
            elif solve_time <= 209.0:
                points = 15
            elif solve_time <= 389.0:
                points = 10
            elif solve_time <= 600.0:
                points = 5
            else:
                points = 0

        total_points += points
        cat_stats[category]["points"] += points

        # Print position result line
        solved_str = f"{GREEN}YES{NC}" if solved else f"{RED}NO{NC}"
        time_str = f"{solve_time:.2f}" if solved else "-"
        points_color = GREEN if points > 0 else NC
        points_str = f"{points_color}{points}{NC}"
        
        engine_move_print = bestmove if bestmove else "None"
        engine_color = GREEN if solved else RED
        
        print(f"{pos_id:<13} | {category:<10} | {correct_move:<7} | {engine_color}{engine_move_print:<7}{NC} | {solved_str:<7} | {time_str:<8} | {points_str:<6}")

    # Calculate final Elo
    estimated_elo = 1900 + total_points

    # Print summary dashboard
    print(f"--------------------------------------------------------------------------------")
    print(f"\n{CYAN}{BOLD}============================== RESULTS SUMMARY =============================={NC}")
    print(f"Positions Solved: {BOLD}{solved_count} / {len(positions)}{NC} ({solved_count/len(positions)*100:.1f}%)")
    print(f"Total Points Scored: {BOLD}{total_points} / 1050{NC}")
    print(f"\n{BOLD}Category Breakdown:{NC}")
    for cat, stats in cat_stats.items():
        score_pct = (stats["solved"] / stats["total"]) * 100
        print(f"  - {cat:<10}: {BOLD}{stats['solved']}/{stats['total']}{NC} solved ({score_pct:.1f}%) | {stats['points']} points")
    
    print(f"\n{MAGENTA}{BOLD}================================================================{NC}")
    print(f"{MAGENTA}{BOLD}             ESTIMATED ENGINE ELO RATING: {GREEN}{BOLD}{estimated_elo} Elo{NC}")
    print(f"{MAGENTA}{BOLD}================================================================{NC}\n")

def main():
    parser = argparse.ArgumentParser(description="Louguet Chess Test II (LCT II) Elo Estimator for UCI Chess Engines")
    parser.add_argument("-b", "--binary", default="target/release/suprah", help="Path to the chess engine binary")
    parser.add_argument("-t", "--timeout", type=int, default=10, help="Timeout in seconds per position (default: 10)")
    parser.add_argument("--build", action="store_true", help="Compile release binary before starting the test")
    
    args = parser.parse_args()

    # Compile if requested
    if args.build:
        print(f"{YELLOW}Compiling optimized release binary...{NC}")
        res = subprocess.run(["cargo", "build", "--release"])
        if res.returncode != 0:
            print(f"{RED}Error: Cargo compilation failed!{NC}")
            sys.exit(1)
        print(f"{GREEN}Success: Compilation completed!{NC}")

    # Verify binary exists
    if not os.path.exists(args.binary):
        print(f"{RED}Error: Binary not found at '{args.binary}'!{NC}")
        print(f"{YELLOW}Please compile the engine with 'cargo build --release' or pass the correct binary path with -b.{NC}")
        sys.exit(1)

    run_uci_test(args.binary, LCT2_POSITIONS, args.timeout)

if __name__ == "__main__":
    main()
