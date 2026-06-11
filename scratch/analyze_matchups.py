import sys
import re
from collections import defaultdict

log_file_path = "/home/mattmagie/mattmagie/mattmagie.log"

# Game session tracker
# A new game starts with "Matt-Magie" start or loaded eng lines.
# We will collect data for each game, then analyze.

class GameData:
    def __init__(self, eng0_path, eng1_path):
        self.eng0_path = eng0_path
        self.eng1_path = eng1_path
        self.eng0_searches = [] # list of (depth, nodes, nps)
        self.eng1_searches = [] # list of (depth, nodes, nps)
        self.result = None

games = []
current_game = None
current_engines = {0: None, 1: None}
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
        
        # Game boundaries
        if "Matt-Magie" in line and "started" in line:
            if current_game:
                games.append(current_game)
            current_game = None
            current_engines = {0: None, 1: None}
            current_search = {
                0: {"depth": 0, "nodes": 0, "nps": 0, "has_info": False},
                1: {"depth": 0, "nodes": 0, "nps": 0, "has_info": False}
            }
            continue
            
        if "loaded eng" in line:
            m = loaded_eng_pattern.search(line)
            if m:
                idx = int(m.group(1))
                path = m.group(2).strip()
                current_engines[idx] = path
                current_search[idx] = {"depth": 0, "nodes": 0, "nps": 0, "has_info": False}
                
                # If both are loaded, we can initialize current_game
                if current_engines[0] and current_engines[1] and not current_game:
                    current_game = GameData(current_engines[0], current_engines[1])
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
                curr = current_search[idx]
                if current_game and curr["has_info"]:
                    if idx == 0:
                        current_game.eng0_searches.append((curr["depth"], curr["nodes"], curr["nps"]))
                    else:
                        current_game.eng1_searches.append((curr["depth"], curr["nodes"], curr["nps"]))
                current_search[idx] = {"depth": 0, "nodes": 0, "nps": 0, "has_info": False}
            continue
            
        # Check game result
        # e.g., "BlackWin", "WhiteWin", "Draw", "Game status != Normal" etc.
        if "BlackWin" in line or "WhiteWin" in line or "Draw" in line:
            if current_game:
                current_game.result = line.strip()

# Add final game if any
if current_game:
    games.append(current_game)

print(f"Parsed {len(games)} total games.")

def analyze_matchup(e1_sub, e2_sub):
    print(f"\n==========================================")
    print(f"Matchup: {e1_sub} vs {e2_sub}")
    print(f"==========================================")
    
    match_games = []
    for g in games:
        if (e1_sub in g.eng0_path and e2_sub in g.eng1_path) or (e2_sub in g.eng0_path and e1_sub in g.eng1_path):
            match_games.append(g)
            
    if not match_games:
        print("No games found for this matchup.")
        return
        
    print(f"Found {len(match_games)} head-to-head games.")
    
    # Accumulate stats
    # e1 stats
    e1_depths, e1_nodes, e1_nps = [], [], []
    # e2 stats
    e2_depths, e2_nodes, e2_nps = [], [], []
    
    for g in match_games:
        if e1_sub in g.eng0_path:
            e1_searches = g.eng0_searches
            e2_searches = g.eng1_searches
        else:
            e1_searches = g.eng1_searches
            e2_searches = g.eng0_searches
            
        e1_depths.extend([s[0] for s in e1_searches])
        e1_nodes.extend([s[1] for s in e1_searches])
        e1_nps.extend([s[2] for s in e1_searches])
        
        e2_depths.extend([s[0] for s in e2_searches])
        e2_nodes.extend([s[1] for s in e2_searches])
        e2_nps.extend([s[2] for s in e2_searches])
        
    if not e1_depths or not e2_depths:
        print("Insufficient search data for stats.")
        return
        
    e1_avg_depth = sum(e1_depths) / len(e1_depths)
    e1_avg_nodes = sum(e1_nodes) / len(e1_nodes)
    e1_avg_nps = sum(e1_nps) / len(e1_nps)
    
    e2_avg_depth = sum(e2_depths) / len(e2_depths)
    e2_avg_nodes = sum(e2_nodes) / len(e2_nodes)
    e2_avg_nps = sum(e2_nps) / len(e2_nps)
    
    print(f"{e1_sub:<15} | Moves: {len(e1_depths):<6} | Avg Depth: {e1_avg_depth:<6.2f} | Avg Nodes: {e1_avg_nodes:<8.1f} | Avg NPS: {e1_avg_nps:<6.1f}")
    print(f"{e2_sub:<15} | Moves: {len(e2_depths):<6} | Avg Depth: {e2_avg_depth:<6.2f} | Avg Nodes: {e2_avg_nodes:<8.1f} | Avg NPS: {e2_avg_nps:<6.1f}")
    
    # Speed comparison
    nps_diff = (e1_avg_nps - e2_avg_nps) / e2_avg_nps * 100
    print(f"Speed comparison: {e1_sub} is {nps_diff:+.1f}% faster than {e2_sub}")
    
    # Depth distribution comparison
    print("\nDepth Distribution:")
    for sub, depths in [(e1_sub, e1_depths), (e2_sub, e2_depths)]:
        dist = defaultdict(int)
        for d in depths:
            dist[d] += 1
        total = len(depths)
        sorted_d = sorted(dist.keys())
        dist_str = ", ".join(f"D{d}: {dist[d]/total*100:.1f}%" for d in sorted_d if dist[d]/total >= 0.01)
        print(f"  {sub:<15}: {dist_str}")

analyze_matchup("0.13.4", "0.13.13")
analyze_matchup("0.13.4", "0.13.8")
analyze_matchup("0.13.4", "0.13.9")
