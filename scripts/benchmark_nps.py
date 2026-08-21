#!/usr/bin/env python3
import subprocess
import argparse
import os
import sys
import time

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)

DEFAULT_POSITIONS = [
    ("Startpos", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"),
    ("Kiwipete", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"),
    ("Middlegame", "r1bqkb1r/pp3ppp/2n1pn2/2pp4/2PP4/2N1PN2/PP3PPP/R1BQKB1R w KQkq - 0 6"),
    ("Sharp Tactical", "r1b1kb1r/pppp1ppp/8/4q3/3n4/8/PPPPBPPP/RNBQK2R w KQkq - 0 1"),
    ("Rook Endgame", "8/5pk1/7p/8/8/3R2P1/r4PKP/8 w - - 0 1"),
    ("Pawn Endgame", "8/8/5k2/p4p2/P4P2/5K2/8/8 w - - 0 1"),
]

def run_single_search(binary_path, fen, depth, timeout_sec=10):
    cmd = f"""(
echo "uci"
echo "setoption name OwnBook value false"
echo "isready"
echo "position fen {fen}"
echo "go depth {depth}"
sleep 1
echo "quit"
) | {binary_path}"""
    
    try:
        res = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            text=True,
            timeout=timeout_sec,
            cwd=PROJECT_ROOT
        )
        stdout = res.stdout
    except subprocess.TimeoutExpired:
        stdout = ""

    max_nps = 0
    total_nodes = 0
    calc_time_ms = 0
    
    for line in stdout.splitlines():
        if line.startswith("info") and "depth" in line:
            tokens = line.split()
            try:
                if "nps" in tokens:
                    nps_val = int(tokens[tokens.index("nps") + 1])
                    if nps_val > max_nps:
                        max_nps = nps_val
                if "nodes" in tokens:
                    nodes_val = int(tokens[tokens.index("nodes") + 1])
                    if nodes_val > total_nodes:
                        total_nodes = nodes_val
                if "time" in tokens:
                    time_val = int(tokens[tokens.index("time") + 1])
                    if time_val > calc_time_ms:
                        calc_time_ms = time_val
            except (ValueError, IndexError):
                pass
                
    return max_nps, total_nodes, calc_time_ms

def benchmark_tags(tags, depth, baseline_tag):
    results = {}
    current_branch = subprocess.check_output(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"], 
        cwd=PROJECT_ROOT, 
        text=True
    ).strip()

    print(f"Starting Multi-Version NPS Benchmark (Search Depth: {depth})...\n")

    for tag in tags:
        print(f"--> Building and evaluating target '{tag}'...")
        try:
            subprocess.run(["git", "checkout", tag, "-q"], cwd=PROJECT_ROOT, check=True)
            subprocess.run(["cargo", "build", "--release", "-q"], cwd=PROJECT_ROOT, check=True)
        except subprocess.CalledProcessError as e:
            print(f"Error checking out / building {tag}: {e}")
            continue

        binary_path = os.path.join(PROJECT_ROOT, "target", "release", "suprah")
        if not os.path.exists(binary_path):
            print(f"Binary not found at {binary_path}")
            continue

        tag_pos_results = {}
        for pos_name, fen in DEFAULT_POSITIONS:
            nps, nodes, t_ms = run_single_search(binary_path, fen, depth)
            tag_pos_results[pos_name] = {
                "nps": nps,
                "nodes": nodes,
                "time_ms": t_ms
            }
        
        nps_list = [v["nps"] for v in tag_pos_results.values() if v["nps"] > 0]
        avg_nps = int(sum(nps_list) / len(nps_list)) if nps_list else 0
        peak_nps = max(nps_list) if nps_list else 0
        
        results[tag] = {
            "positions": tag_pos_results,
            "avg_nps": avg_nps,
            "peak_nps": peak_nps
        }

    # Restore initial branch
    print(f"\nRestoring workspace to branch '{current_branch}'...")
    subprocess.run(["git", "checkout", current_branch, "-q"], cwd=PROJECT_ROOT)
    subprocess.run(["cargo", "build", "--release", "-q"], cwd=PROJECT_ROOT)

    # Print Summary Markdown Table
    print("\n" + "=" * 80)
    print("## NPS Benchmark Comparison Results")
    print("=" * 80 + "\n")
    
    baseline_avg = results.get(baseline_tag, {}).get("avg_nps", 0) if baseline_tag in results else 0
    if baseline_avg == 0 and results:
        baseline_tag = list(results.keys())[0]
        baseline_avg = results[baseline_tag]["avg_nps"]

    # Table 1: Overall Summary
    print("### Overall Throughput Summary")
    print(f"| Version / Tag | Average NPS | Peak NPS | vs Baseline ({baseline_tag}) |")
    print("| :--- | :--- | :--- | :--- |")
    for tag, data in results.items():
        avg = data["avg_nps"]
        peak = data["peak_nps"]
        if baseline_avg > 0:
            diff_pct = ((avg - baseline_avg) / baseline_avg) * 100
            diff_str = f"{diff_pct:+.1f}%"
            if diff_pct > 0.5:
                diff_str = f"**{diff_str}**"
        else:
            diff_str = "Baseline" if tag == baseline_tag else "-"
        print(f"| `{tag}` | {avg:,} NPS | {peak:,} NPS | {diff_str} |")

    # Table 2: Position Breakdown
    print("\n### Position Breakdown (NPS in kNps)")
    header = "| Version | " + " | ".join([p[0] for p in DEFAULT_POSITIONS]) + " |"
    sep = "| :--- | " + " | ".join([":---" for _ in DEFAULT_POSITIONS]) + " |"
    print(header)
    print(sep)
    for tag, data in results.items():
        row_vals = []
        for pos_name, _ in DEFAULT_POSITIONS:
            pos_nps = data["positions"].get(pos_name, {}).get("nps", 0)
            row_vals.append(f"{pos_nps:,}")
        print(f"| `{tag}` | " + " | ".join(row_vals) + " |")
    print("\n")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Multi-Version NPS Performance Benchmark for Suprah Chess Engine")
    parser.add_argument("tags", nargs="+", help="List of git tags or branches to compare (e.g. v0.25.0 v0.27.2 v0.27.5 master)")
    parser.add_argument("-d", "--depth", type=int, default=8, help="Search depth for benchmark (default: 8)")
    parser.add_argument("-b", "--baseline", default="v0.27.2", help="Baseline tag to compare against (default: v0.27.2)")
    args = parser.parse_args()
    
    benchmark_tags(args.tags, args.depth, args.baseline)
