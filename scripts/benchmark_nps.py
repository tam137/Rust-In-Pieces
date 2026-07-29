#!/usr/bin/env python3
import subprocess
import argparse

def benchmark_tags(tags):
    for tag in tags:
        print(f"Benchmarking {tag}...")
        
        # 1. Checkout and Build
        subprocess.run(["git", "checkout", tag, "-q"], check=True)
        subprocess.run(["cargo", "build", "--release", "-q"], check=True)
        
        # 2. Piped UCI Commands (Disable opening book to enforce a real search)
        cmd = '''(
        echo "uci"
        echo "setoption name OwnBook value false"
        echo "isready"
        echo "position fen r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
        echo "go depth 8"
        sleep 3
        echo "quit"
        ) | ./target/release/suprah'''
        
        p = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        
        # 3. Parse highest NPS from target depth
        max_nps = 0
        for line in p.stdout.split("\n"):
            if "info depth 7" in line and "nps" in line:
                parts = line.split()
                if "nps" in parts:
                    idx = parts.index("nps")
                    nps = int(parts[idx+1])
                    if nps > max_nps:
                        max_nps = nps
                    
        print(f"Result for {tag}: {max_nps} NPS\n")

    # Cleanup: Return to master
    print("Cleaning up... returning to master branch.")
    subprocess.run(["git", "checkout", "master", "-q"])

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Benchmark Engine NPS across multiple git tags/branches.")
    parser.add_argument("tags", nargs="+", help="List of git tags or branches to compare (e.g., v0.22.9 v0.22.10)")
    args = parser.parse_args()
    
    benchmark_tags(args.tags)
