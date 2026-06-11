import sys
import re
from collections import defaultdict

log_file_path = "/home/mattmagie/mattmagie/mattmagie.log"

current_engines = {0: None, 1: None}
stats = defaultdict(list)
current_search = {
    0: {"depth": 0, "nodes": 0, "nps": 0, "has_info": False},
    1: {"depth": 0, "nodes": 0, "nps": 0, "has_info": False}
}

loaded_eng_pattern = re.compile(r"loaded eng([01]) \d+: (.+)")
bestmove_pattern = re.compile(r"(\d)_bestmove")

total_lines = 0

with open(log_file_path, "r", encoding="utf-8", errors="ignore") as f:
    for line in f:
        total_lines += 1
        if "loaded eng" in line:
            m = loaded_eng_pattern.search(line)
            if m:
                idx = int(m.group(1))
                path = m.group(2).strip()
                current_engines[idx] = path
                current_search[idx] = {"depth": 0, "nodes": 0, "nps": 0, "has_info": False}
            continue
            
        if "_info depth" in line:
            idx_match = re.search(r"(\d)_info", line)
            if idx_match:
                idx = int(idx_match.group(1))
                depth_match = re.search(r"\bdepth (\d+)\b", line)
                nodes_match = re.search(r"\bnodes (\d+)\b", line)
                nps_match = re.search(r"\bnps (\d+)\b", line)
                
                if depth_match and nodes_match and nps_match:
                    depth = int(depth_match.group(1))
                    nodes = int(nodes_match.group(1))
                    nps = int(nps_match.group(1))
                    
                    curr = current_search[idx]
                    curr["depth"] = max(curr["depth"], depth)
                    curr["nodes"] = max(curr["nodes"], nodes)
                    curr["nps"] = nps
                    curr["has_info"] = True
            continue
            
        if "_bestmove" in line:
            m = bestmove_pattern.search(line)
            if m:
                idx = int(m.group(1))
                engine = current_engines[idx]
                curr = current_search[idx]
                if engine and curr["has_info"]:
                    stats[engine].append((curr["depth"], curr["nodes"], curr["nps"]))
                current_search[idx] = {"depth": 0, "nodes": 0, "nps": 0, "has_info": False}
            continue

print(f"Processed {total_lines} lines.")
print("Engine Statistics:")
print(f"{'Engine':<30} | {'Moves':<8} | {'Avg Depth':<10} | {'Avg Nodes':<12} | {'Avg NPS':<10}")
print("-" * 80)
for eng, data in sorted(stats.items()):
    if not data:
        continue
    moves = len(data)
    avg_depth = sum(d[0] for d in data) / moves
    avg_nodes = sum(d[1] for d in data) / moves
    avg_nps = sum(d[2] for d in data) / moves
    print(f"{eng:<30} | {moves:<8} | {avg_depth:<10.2f} | {avg_nodes:<12.1f} | {avg_nps:<10.1f}")
