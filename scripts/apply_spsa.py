import json
import re

with open('tuning/spsa_state_remote.json', 'r') as f:
    state = json.load(f)

theta = state['theta']

with open('src/config.rs', 'r') as f:
    config_code = f.read()

# We only want to replace within the `pub fn new() -> Config {` block up to its end.
new_fn_start = config_code.find('pub fn new() -> Config {')
new_fn_end = config_code.find('}', config_code.find('log_path: std::sync::Arc::from("")'))

if new_fn_start == -1 or new_fn_end == -1:
    print("Could not find Config::new() block!")
    exit(1)

new_fn_body = config_code[new_fn_start:new_fn_end]

for key, value in theta.items():
    rounded_val = round(value)
    # Search for `key: <old_val>,`
    pattern = r'(\s*' + key + r'\s*:\s*)-?\d+(,)'
    new_fn_body = re.sub(pattern, r'\g<1>' + str(rounded_val) + r'\g<2>', new_fn_body)

# Replace the old body with the new body
new_config_code = config_code[:new_fn_start] + new_fn_body + config_code[new_fn_end:]

with open('src/config.rs', 'w') as f:
    f.write(new_config_code)

print("Applied parameters successfully.")
