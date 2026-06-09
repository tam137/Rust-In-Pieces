#!/usr/bin/env python3
import os
import sys
import json
import subprocess
import re

# ANSI colors
GREEN = "\033[0;32m"
RED = "\033[0;31m"
YELLOW = "\033[0;33m"
CYAN = "\033[0;36m"
MAGENTA = "\033[0;35m"
BOLD = "\033[1m"
NC = "\033[0m"

def fetch_all_data(server_ip):
    # Bundles all commands to execute in a single SSH connection
    script = """
echo ""
echo "=== CMD ==="
ps aux | grep spsa_tuner.py | grep -v grep || echo "NO_CMD"
echo ""
echo "=== TMUX ==="
tmux capture-pane -t spsa_tuning -p 2>/dev/null || echo "NO_TMUX"
echo ""
echo "=== STATE ==="
cat /root/mattmagie/tuning/spsa_state.json 2>/dev/null || echo "NO_STATE"
echo ""
echo "=== PARAMS ==="
cat /root/mattmagie/tuning/parameters.json 2>/dev/null || echo "NO_PARAMS"
echo ""
echo "=== LOGS ==="
files=$(ls -t /root/mattmagie/tuning/enginelogs 2>/dev/null | grep '\.log$' | head -n 3)
for f in $files; do
  echo ""
  echo "--- FILE: $f ---"
  cat "/root/mattmagie/tuning/enginelogs/$f" 2>/dev/null
  echo ""
  echo "--- END_FILE ---"
done
"""
    ssh_opts = [
        "-o", "ControlMaster=auto",
        "-o", "ControlPath=~/.ssh/control-%r@%h:%p",
        "-o", "ControlPersist=10m",
        "-o", "ConnectTimeout=10"
    ]
    res = subprocess.run(["ssh"] + ssh_opts + [f"root@{server_ip}", script], capture_output=True, text=True)
    if res.returncode != 0:
        print(f"{RED}Error: SSH command failed with return code {res.returncode}{NC}")
        print(f"Stderr: {res.stderr.strip()}")
        return None
    return res.stdout

def parse_sections(stdout):
    sections = {}
    current_section = None
    current_lines = []
    
    for line in stdout.splitlines():
        if line.startswith("=== ") and line.endswith(" ==="):
            if current_section is not None:
                sections[current_section] = "\n".join(current_lines)
            current_section = line[4:-4]
            current_lines = []
        else:
            if current_section is not None:
                current_lines.append(line)
                
    if current_section is not None:
        sections[current_section] = "\n".join(current_lines)
        
    return sections

def parse_logs(logs_section):
    log_files_content = {}
    current_file = None
    file_lines = []
    
    for line in logs_section.splitlines():
        if line.startswith("--- FILE: ") and line.endswith(" ---"):
            current_file = line[10:-4]
            file_lines = []
        elif line.strip() == "--- END_FILE ---":
            if current_file:
                log_files_content[current_file] = "\n".join(file_lines)
                current_file = None
        else:
            if current_file:
                file_lines.append(line)
                
    return log_files_content

def main():
    server_ip = os.environ.get("EODSERVERIP", "135.181.27.105")
    print(f"{CYAN}================================================================{NC}")
    print(f"{CYAN}{BOLD}              SPSA TUNING STATUS & VERIFICATION REPORT          {NC}")
    print(f"{CYAN}================================================================{NC}")
    print(f"Target Server: {BOLD}{server_ip}{NC}\n")

    print(f"{YELLOW}{BOLD}[1/4] Fetching remote SPSA data in a single SSH session...{NC}")
    raw_data = fetch_all_data(server_ip)
    if not raw_data:
        print(f"{RED}Error: Failed to fetch SPSA data from server.{NC}\n")
        sys.exit(1)
        
    sections = parse_sections(raw_data)
    
    # 1. Verify tmux & command
    cmd_output = sections.get("CMD", "")
    tmux_output = sections.get("TMUX", "")
    
    # Parse active parameters from process list or tmux
    full_cmd = cmd_output + "\n" + tmux_output
    match = re.search(r'--params\s+([a-zA-Z0-9_,]+)', full_cmd)
    if match:
        active_params = [p.strip() for p in match.group(1).split(",") if p.strip()]
    else:
        active_params = None

    if "NO_TMUX" in tmux_output or not tmux_output.strip():
        print(f"{RED}Error: Could not capture tmux pane 'spsa_tuning'. Is the tuner running?{NC}\n")
    else:
        print(f"{GREEN}Success: Tmux pane captured!{NC}")
        print("Last 10 lines of tmux output:")
        lines = [l for l in tmux_output.split("\n") if l.strip()]
        for line in lines[-10:]:
            print(f"  > {line}")
        print()
        
        if active_params:
            print(f"Detected active parameters from SPSA command line:")
            print(f"  {BOLD}{active_params}{NC}\n")
        else:
            print(f"No --params filter detected. Tuning ALL parameters by default.\n")

    # 2. SPSA State
    print(f"{YELLOW}{BOLD}[2/4] Verifying remote SPSA state...{NC}")
    state_json_str = sections.get("STATE", "")
    state_data = None
    if "NO_STATE" in state_json_str or not state_json_str.strip():
        print(f"{YELLOW}spsa_state.json not found on server. Checking fallback to parameters.json...{NC}")
        params_json_str = sections.get("PARAMS", "")
        if "NO_PARAMS" not in params_json_str and params_json_str.strip():
            try:
                params_data = json.loads(params_json_str)
                state_data = {
                    "k": 1,
                    "theta": {k: float(v["value"]) for k, v in params_data.items()}
                }
                print(f"{GREEN}Success: Loaded baseline parameters from parameters.json (Iteration 1){NC}")
                print(f"Number of parameters tracked: {len(state_data.get('theta', {}))}\n")
            except Exception as e:
                print(f"{RED}Error parsing parameters.json fallback: {e}{NC}\n")
        else:
            print(f"{RED}Error: Could not read parameters.json fallback on server!{NC}\n")
    else:
        try:
            state_data = json.loads(state_json_str)
            print(f"{GREEN}Success: Read state at Iteration {state_data.get('k')}{NC}")
            print(f"Number of parameters tracked: {len(state_data.get('theta', {}))}\n")
        except Exception as e:
            print(f"{RED}Error parsing spsa_state.json: {e}{NC}\n")

    # Load local parameters.json for defaults
    local_params_path = "tuning/parameters.json"
    local_params = {}
    if os.path.exists(local_params_path):
        try:
            with open(local_params_path, "r") as f:
                local_params = json.load(f)
        except Exception as e:
            print(f"{RED}Warning: Could not read local parameters.json: {e}{NC}")

    # 3. Check engine logs
    print(f"{YELLOW}{BOLD}[3/4] Parsing and verifying engine logs...{NC}")
    logs_section = sections.get("LOGS", "")
    log_files_content = parse_logs(logs_section)
    
    if not log_files_content:
        print(f"{RED}Error: No engine log files parsed from remote SPSA output.{NC}\n")
        sys.exit(1)
        
    print(f"Found {len(log_files_content)} recent log files. Verifying:")
    
    successful_reads = 0
    mismatches_found = False
    logpath_missing_found = False
    params_missing_found = False

    for name, log_content in log_files_content.items():
        print(f"\n  Checking log file: {BOLD}{name}{NC}")
        if not log_content.strip():
            print(f"    {YELLOW}Warning: Log content is empty (might be newly initialized){NC}")
            continue

        successful_reads += 1

        # Check for Received Option
        has_logpath_opt = "Received option: logpath" in log_content
        if has_logpath_opt:
            print(f"    {GREEN}✓ Found logpath option initialization{NC}")
        else:
            print(f"    {RED}✗ Missing logpath option initialization{NC}")
            logpath_missing_found = True

        # Parse parameters
        param_pattern = re.compile(r"^\s*([a-zA-Z0-9_]+):\s*(-?\d+)\s*$")
        logged_params = {}
        in_params_section = False
        for line in log_content.split("\n"):
            if "Current Engine Parameters:" in line:
                in_params_section = True
                continue
            if in_params_section:
                m = param_pattern.match(line)
                if m:
                    param_name = m.group(1)
                    param_val = int(m.group(2))
                    logged_params[param_name] = param_val
                elif line.strip() == "":
                    in_params_section = False

        if not logged_params:
            print(f"    {RED}✗ Could not parse any engine parameters from log file!{NC}")
            params_missing_found = True
            continue
        
        print(f"    {GREEN}✓ Parsed {len(logged_params)} parameters from engine log{NC}")

        # Check parameter values
        if state_data:
            theta = state_data.get("theta", {})
            
            # Check inactive parameters (must equal SPSA state theta / default)
            if active_params is not None:
                inactive_ok = True
                for p_name, val in logged_params.items():
                    if p_name not in active_params:
                        expected_val = int(round(theta.get(p_name, local_params.get(p_name, {}).get("value", 0))))
                        if val != expected_val:
                            print(f"      {RED}✗ Inactive parameter mismatch: {p_name} = {val} (expected {expected_val}){NC}")
                            inactive_ok = False
                            mismatches_found = True
                if inactive_ok:
                    print(f"      {GREEN}✓ Verified all inactive parameters are fixed at baseline defaults{NC}")

            # Display active parameters and their values/deviations
            print(f"    Active parameters in log:")
            active_list = active_params if active_params is not None else sorted(list(theta.keys()))
            for p_name in active_list:
                if p_name in logged_params:
                    val = logged_params[p_name]
                    base_val = theta.get(p_name, local_params.get(p_name, {}).get("value", 0))
                    dev = val - base_val
                    print(f"      - {p_name}: {val} (baseline: {base_val:.2f}, diff: {dev:+.2f})")
                else:
                    print(f"      - {p_name}: {RED}NOT FOUND IN LOG{NC}")
                    params_missing_found = True

    print()
    # 4. Plausibility Summary
    print(f"{YELLOW}{BOLD}[4/4] Plausibility Verification Summary...{NC}")
    if successful_reads == 0:
        print(f"{RED}{BOLD}✗ TECHNICAL CHECK FAILED:{NC} All engine log files were unreadable (empty or locked).")
    elif logpath_missing_found or params_missing_found or mismatches_found:
        print(f"{RED}{BOLD}✗ TECHNICAL CHECK FAILED:{NC} Mismatches or errors were found during SPSA log analysis.")
    else:
        print(f"{GREEN}{BOLD}✓ TECHNICAL CHECK PASSED:{NC} Remote SPSA parameters are being updated, filtered, and logged correctly.")
    print(f"{CYAN}================================================================{NC}")

if __name__ == "__main__":
    main()
