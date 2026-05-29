import re
import json

lines = []
with open("/home/tam137/git/suprah/src/config.rs", "r") as f:
    for _ in range(158):
        f.readline()
    for _ in range(239 - 159 + 1):
        lines.append(f.readline())

params = {}
for line in lines:
    line = line.strip()
    if line.startswith("pub") or line.startswith("//") or line == "":
        continue
    
    # looking for "name: value,"
    match = re.match(r'([a-zA-Z0-9_]+):\s*(-?\d+),', line)
    if match:
        name = match.group(1)
        val = int(match.group(2))
        if name in ["delta_pruning_margin", "killer_move_1_rank_bonus", "killer_move_2_rank_bonus", "counter_move_rank_bonus", "history_max_threshold"]:
            continue
        params[name] = {
            "value": val,
            "min": max(0, val - 100) if val > 0 else val - 100,
            "max": val + 100
        }

with open("/home/tam137/git/suprah/tuning/parameters.json", "w") as f:
    json.dump(params, f, indent=4)
print("Extracted", len(params), "parameters")
