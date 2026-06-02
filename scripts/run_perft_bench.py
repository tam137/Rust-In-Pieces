import subprocess

def run_bench():
    # Start the compiled release binary directly
    p = subprocess.Popen(
        ["./target/release/suprah"], 
        stdin=subprocess.PIPE, 
        stdout=subprocess.PIPE, 
        text=True
    )
    
    # Send UCI setup commands to bypass the opening book
    commands = (
        "uci\n"
        "setoption name EnableEasyMove value false\n"
        "isready\n"
        "position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 5\n"
        "go depth 10\n"
    )
    p.stdin.write(commands)
    p.stdin.flush()
    
    # Store the info output per depth
    depth_stats = {}
    
    while True:
        line = p.stdout.readline()
        if not line:
            break
        line = line.strip()
        print(line)
        
        # Parse info depth output
        if line.startswith("info depth"):
            parts = line.split()
            try:
                d_idx = parts.index("depth")
                depth = int(parts[d_idx + 1])
                
                # Check for score, time, nodes, nps
                time_val = "-"
                nodes_val = "-"
                nps_val = "-"
                
                if "time" in parts:
                    t_idx = parts.index("time")
                    time_val = parts[t_idx + 1]
                if "nodes" in parts:
                    n_idx = parts.index("nodes")
                    nodes_val = parts[n_idx + 1]
                if "nps" in parts:
                    np_idx = parts.index("nps")
                    nps_val = parts[np_idx + 1]
                    
                depth_stats[depth] = (time_val, nodes_val, nps_val)
            except ValueError:
                pass
                
        if "bestmove" in line:
            break
            
    p.stdin.write("quit\n")
    p.stdin.flush()
    p.wait()
    
    print("\n--- Generated Markdown Table ---")
    print("| Depth | Time | Nodes | NPS |")
    print("| :--- | :--- | :--- | :--- |")
    print("| 1 | - | - | - |")
    for d in sorted(depth_stats.keys()):
        if d == 1:
            continue
        t, n, nps = depth_stats[d]
        # Format time nicely
        if t != "-":
            t_ms = int(t)
            if t_ms == 0:
                t_str = "< 1 ms"
            else:
                t_str = f"{t_ms} ms"
        else:
            t_str = "-"
            
        # Format nodes and NPS with commas
        n_str = f"{int(n):,}" if n != "-" else "-"
        nps_str = f"{int(nps):,}" if nps != "-" else "-"
        
        print(f"| {d} | {t_str} | {n_str} | {nps_str} |")

if __name__ == "__main__":
    run_bench()
