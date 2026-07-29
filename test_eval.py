import subprocess
import time

def evaluate_position(engine_path, fen):
    p = subprocess.Popen([engine_path], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    p.stdin.write("uci\n")
    p.stdin.write(f"position fen {fen}\n")
    p.stdin.write("go depth 8\n")
    p.stdin.flush()
    
    start_time = time.time()
    nodes = 0
    nps = 0
    time_ms = 0
    while True:
        line = p.stdout.readline()
        if "info depth" in line:
            parts = line.split()
            if "nodes" in parts:
                nodes = int(parts[parts.index("nodes")+1])
            if "nps" in parts:
                nps = int(parts[parts.index("nps")+1])
            if "time" in parts:
                time_ms = int(parts[parts.index("time")+1])
        if "bestmove" in line or (time.time() - start_time > 15):
            break
    p.stdin.write("quit\n")
    p.stdin.flush()
    p.wait()
    return nodes, nps, time_ms

subprocess.run(["git", "checkout", "master"])
subprocess.run(["cargo", "build", "--release"])
# Kiwipete position (not in standard book)
fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
nodes_master, nps_master, time_master = evaluate_position("./target/release/suprah", fen)

subprocess.run(["git", "checkout", "HEAD~1"])
subprocess.run(["cargo", "build", "--release"])
nodes_old, nps_old, time_old = evaluate_position("./target/release/suprah", fen)

print(f"Master: NPS = {nps_master}, Nodes = {nodes_master}, Time = {time_master} ms")
print(f"Old:    NPS = {nps_old}, Nodes = {nodes_old}, Time = {time_old} ms")
subprocess.run(["git", "checkout", "master"])

