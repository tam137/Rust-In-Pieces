import re
import json

config_file = "src/config.rs"

# Parse all fields and their types from the Config struct
with open(config_file, "r") as f:
    content = f.read()

struct_match = re.search(r"pub struct Config \{(.*?)\}", content, re.DOTALL)
if not struct_match:
    print("Could not find Config struct")
    exit(1)

struct_body = struct_match.group(1)

fields = {}
for line in struct_body.split("\n"):
    line = line.strip()
    if line.startswith("pub "):
        parts = line[4:].split(":")
        if len(parts) == 2:
            name = parts[0].strip()
            type_str = parts[1].strip().strip(",")
            fields[name] = type_str

# Parse defaults
default_match = re.search(r"impl Default for Config \{.*?fn default\(\) -> Self \{.*?Config \{(.*?)\}.*?\}", content, re.DOTALL)
if not default_match:
    print("Could not find Config defaults")
    exit(1)

default_body = default_match.group(1)
defaults = {}
for line in default_body.split("\n"):
    line = line.strip()
    if ":" in line and not line.startswith("//"):
        parts = line.split(":")
        if len(parts) >= 2:
            name = parts[0].strip()
            val_str = parts[1].split(",")[0].strip()
            try:
                defaults[name] = float(val_str)
            except ValueError:
                pass

# Non-eval parameters to exclude from tuning
exclude = {"search_threads", "tt_size_mb", "move_overhead_ms"}

parameters = {}
for name, typ in fields.items():
    if name in exclude:
        continue
    
    val = defaults.get(name, 0.0)
    
    # Calculate min and max safely
    _min = 0.0
    _max = val + max(50.0, val * 1.5)
    
    # Allow negative values for maluses or anything that might dip
    if "malus" in name or val <= 0:
        _min = min(-50.0, val - 50.0)
    
    parameters[name] = {
        "value": val,
        "min": _min,
        "max": _max
    }

with open("tuning/parameters.json", "w") as f:
    json.dump(parameters, f, indent=4)

print(f"Extracted {len(parameters)} parameters to tuning/parameters.json")
