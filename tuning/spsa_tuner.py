import json
import os
import random
import subprocess
import time
import csv
import argparse
import concurrent.futures
import threading

import sys

class SPSATuner:
    def __init__(self, params_file, state_file, history_file, engine_path, mm_path, games_per_iter=2500, workers=8, time_ms=1000, inc_ms=10, mutate_pct=10.0, lr=5.0, logpath="", active_params=None, book_path="", group_name="all"):
        self.params_file = params_file
        self.state_file = state_file
        self.history_file = history_file
        self.engine_path = engine_path
        self.mm_path = mm_path
        self.games_per_iter = games_per_iter
        self.workers = workers
        self.time_ms = time_ms
        self.inc_ms = inc_ms
        self.mutate_pct = mutate_pct
        self.logpath = logpath
        self.book_path = book_path
        self.group_name = group_name
        
        # Learning rate percentage (e.g. 20.0 = 20% of mutation step size)
        self.lr_pct = float(lr)

        with open(self.params_file, "r") as f:
            self.param_defs = json.load(f)
            
        self.param_names = list(self.param_defs.keys())
        if active_params:
            self.active_params = [p.strip() for p in active_params if p.strip()]
            for p in self.active_params:
                if p not in self.param_defs:
                    raise ValueError(f"Parameter '{p}' not found in parameters.json")
        else:
            self.active_params = self.param_names.copy()
        
        self.k = 1
        self.theta = {k: float(v["value"]) for k, v in self.param_defs.items()}
        
        # Load state if exists
        if os.path.exists(self.state_file):
            with open(self.state_file, "r") as f:
                state = json.load(f)
                self.k = state.get("k", 1)
                loaded_theta = state.get("theta", {})
                for k in self.param_names:
                    if k in loaded_theta:
                        self.theta[k] = float(loaded_theta[k])
                print(f"Loaded state from iteration {self.k}")
        else:
            # Initialize history file
            with open(self.history_file, "w", newline="") as f:
                writer = csv.writer(f)
                writer.writerow(["Iteration", "Group", "Score"] + self.param_names)

    def format_uci_options(self, theta_rounded):
        return ",".join([f"{k}={v}" for k, v in theta_rounded.items()])

    def run_match_batch(self, theta_plus, theta_minus):
        # Round thetas for UCI
        t_plus_rounded = {k: int(round(v)) for k, v in theta_plus.items()}
        t_minus_rounded = {k: int(round(v)) for k, v in theta_minus.items()}
        
        opts_plus = self.format_uci_options(t_plus_rounded)
        opts_minus = self.format_uci_options(t_minus_rounded)

        if self.logpath:
            # Fallback to local enginelogs if target directory is not writeable
            resolved_logpath = self.logpath
            try:
                os.makedirs(resolved_logpath, exist_ok=True)
            except Exception:
                resolved_logpath = "enginelogs"
                os.makedirs(resolved_logpath, exist_ok=True)
            
            logpath_opt = f"logpath={resolved_logpath}"
            opts_plus = f"{opts_plus},{logpath_opt}" if opts_plus else logpath_opt
            opts_minus = f"{opts_minus},{logpath_opt}" if opts_minus else logpath_opt

        if self.book_path:
            book_opts = f"bookfile={self.book_path},ownbook=true"
            opts_plus = f"{opts_plus},{book_opts}" if opts_plus else book_opts
            opts_minus = f"{opts_minus},{book_opts}" if opts_minus else book_opts
        
        # We need to run matt-magie `games_per_iter` times.
        # matt-magie args: engine_0 engine_1 logfile pgn_path event site round time_per_game inc_per_move log_on debug_on eng_0_opts eng_1_opts
        
        pgn_file = f"tuning/games_{self.k}.pgn"
        log_file = f"tuning/mm_{self.k}.log"
        
        # We can run them sequentially or in parallel. Since matt-magie writes to a single PGN file, 
        # parallel writing might corrupt it if pgn.save() is not atomic, but since we just append, 
        # it might interleave. Let's run a thread pool but with distinct PGNs or just sequentially.
        # We will use sequential for safety or ThreadPool with distinct files.
        # Actually, matt-magie is quite fast for 2s + 100ms.
        
        print(f"Iter {self.k}: Running {self.games_per_iter} games...")
        
        def run_single_game(i):
            tmp_pgn = f"tmp_{self.k}_{i}.pgn"
            # Alternate colors: even game index plays opts_plus as White (engine_0),
            # odd game index plays opts_minus as White (engine_0).
            is_plus_white = (i % 2 == 0)
            e0_opts = opts_plus if is_plus_white else opts_minus
            e1_opts = opts_minus if is_plus_white else opts_plus
            cmd = [
                self.mm_path,
                self.engine_path,
                self.engine_path,
                "/dev/null",
                tmp_pgn,
                "SPSA_Tuning",
                "Local",
                str(i),
                str(self.time_ms),
                str(self.inc_ms),
                "log_off",
                "debug_off",
                e0_opts,
                e1_opts
            ]
            subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            return tmp_pgn

        wins = 0
        losses = 0
        draws = 0
        
        completed_games = 0
        lock = threading.Lock()
        
        def run_single_game_wrapper(i):
            res = run_single_game(i)
            with lock:
                nonlocal completed_games
                completed_games += 1
                percent = (completed_games / self.games_per_iter) * 100
                print(f"\rProgress: {percent:.1f}% ({completed_games}/{self.games_per_iter})", end="", flush=True)
            return res

        # Run in parallel
        with concurrent.futures.ThreadPoolExecutor(max_workers=self.workers) as executor:
            pgn_files = list(executor.map(run_single_game_wrapper, range(self.games_per_iter)))
        print("") # newline after progress bar
            
        # Aggregate results
        for pf in pgn_files:
            if os.path.exists(pf):
                filename = os.path.basename(pf)
                # Extract game index from filename e.g. "tmp_1_45.pgn"
                parts = filename.replace(".pgn", "").split("_")
                game_idx = int(parts[-1])
                is_plus_white = (game_idx % 2 == 0)

                with open(pf, "r") as f:
                    content = f.read()
                    if "[Result \"1-0\"]" in content:
                        # White won
                        if is_plus_white:
                            wins += 1    # Plus won
                        else:
                            losses += 1  # Minus won
                    elif "[Result \"0-1\"]" in content:
                        # Black won
                        if is_plus_white:
                            losses += 1  # Minus won
                        else:
                            wins += 1    # Plus won
                    elif "[Result \"1/2-1/2\"]" in content:
                        draws += 1
                os.remove(pf)
                
        print(f"Results: +{wins} ={draws} -{losses}")
        total = wins + draws + losses
        if total == 0:
            return 0.5
            
        return (wins + 0.5 * draws) / total

    def step(self):
        # Bernoulli +-1
        delta = {k: random.choice([-1, 1]) for k in self.active_params}
        
        # Calculate integer perturbation steps based on percentage
        step_sizes = {}
        for k in self.active_params:
            base_val = abs(self.theta[k])
            step = max(1.0, round(base_val * (self.mutate_pct / 100.0)))
            step_sizes[k] = step

        # Perturb only active parameters and clamp to min/max bounds
        theta_plus = {}
        theta_minus = {}
        for k in self.param_names:
            _min = self.param_defs[k]["min"]
            _max = self.param_defs[k]["max"]
            if k in self.active_params:
                theta_plus[k] = max(_min, min(_max, self.theta[k] + step_sizes[k] * delta[k]))
                theta_minus[k] = max(_min, min(_max, self.theta[k] - step_sizes[k] * delta[k]))
            else:
                theta_plus[k] = max(_min, min(_max, self.theta[k]))
                theta_minus[k] = max(_min, min(_max, self.theta[k]))
        
        score = self.run_match_batch(theta_plus, theta_minus)
        
        # Binary direction based on win (> 0.50), loss (< 0.50), or tie (== 0.50)
        if score > 0.50:
            direction = 1.0
        elif score < 0.50:
            direction = -1.0
        else:
            direction = 0.0
        
        # Update parameters by (lr_pct % of mutation step size) in the winning direction
        lr_ratio = self.lr_pct / 100.0
        for k in self.active_params:
            mutation_step = step_sizes[k] * delta[k]
            update = lr_ratio * mutation_step * direction
            self.theta[k] += update
            
            # Apply bounds
            _min = self.param_defs[k]["min"]
            _max = self.param_defs[k]["max"]
            self.theta[k] = max(_min, min(_max, self.theta[k]))
            
        # Save state
        self.k += 1
        with open(self.state_file, "w") as f:
            json.dump({"k": self.k, "theta": self.theta}, f, indent=4)
            
        # Save history
        with open(self.history_file, "a", newline="") as f:
            writer = csv.writer(f)
            row = [self.k - 1, self.group_name, score] + [self.theta[k] for k in self.param_names]
            writer.writerow(row)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="SPSA Parameter Tuner for Suprah Chess Engine")
    parser.add_argument("--group", required=True, help="Mandatory tuning group (e.g. pawns, king_safety, pieces_and_outposts, rooks, tactics_and_threats, search_and_ordering, or all)")
    parser.add_argument("--engine", required=True)
    parser.add_argument("--mm", required=True)
    parser.add_argument("--games", type=int, default=2500)
    parser.add_argument("--workers", type=int, default=8, help="Number of parallel games to run simultaneously")
    parser.add_argument("--time", type=int, default=1, help="Time per game in seconds")
    parser.add_argument("--inc", type=int, default=10, help="Increment per move in milliseconds")
    parser.add_argument("--mutate", type=float, default=10.0, help="Perturbation percentage per parameter (e.g., 10 for 10%%)")
    parser.add_argument("--lr", type=float, default=5.0, help="Learning rate as a percentage of mutation step size (e.g. 5 for 5%%)")
    parser.add_argument("--logpath", default="enginelogs")
    parser.add_argument("--book", default="", help="Path to PolyGlot opening book (.bin file)")
    parser.add_argument("--params", default="", help="Optional explicit comma-separated list of parameters to tune (overrides group)")
    parser.add_argument("--iters", type=int, default=100, help="Number of SPSA iterations to run")
    args = parser.parse_args()
    
    # Load groups.json
    groups_file = os.path.join(os.path.dirname(__file__), "groups.json")
    if not os.path.exists(groups_file):
        groups_file = "groups.json"
        
    if os.path.exists(groups_file):
        with open(groups_file, "r") as f:
            available_groups = json.load(f)
    else:
        available_groups = {}

    if args.group not in available_groups and not args.params:
        print(f"Error: Invalid group '{args.group}'.")
        print("Available groups in groups.json:")
        for g_name in available_groups.keys():
            print(f"  - {g_name}")
        sys.exit(1)

    if args.params:
        active_params = [p.strip() for p in args.params.split(",") if p.strip()]
        group_name = args.group or "custom"
    else:
        active_params = available_groups[args.group]
        group_name = args.group

    print(f"Starting SPSA Tuner for group '{group_name}' with {len(active_params)} active parameters.")

    script_dir = os.path.dirname(os.path.abspath(__file__))
    params_file = os.path.join(script_dir, "parameters.json") if os.path.exists(os.path.join(script_dir, "parameters.json")) else "parameters.json"
    state_file = os.path.join(script_dir, "spsa_state.json") if os.path.exists(os.path.join(script_dir, "spsa_state.json")) else "spsa_state.json"
    history_file = os.path.join(script_dir, "spsa_history.csv") if os.path.exists(os.path.join(script_dir, "spsa_history.csv")) else "spsa_history.csv"

    tuner = SPSATuner(
        params_file=params_file,
        state_file=state_file,
        history_file=history_file,
        engine_path=args.engine,
        mm_path=args.mm,
        games_per_iter=args.games,
        workers=args.workers,
        time_ms=args.time * 1000,
        inc_ms=args.inc,
        mutate_pct=args.mutate,
        lr=args.lr,
        logpath=args.logpath,
        active_params=active_params,
        book_path=args.book,
        group_name=group_name
    )
    
    # Run SPSA iterations
    for _ in range(args.iters):
        tuner.step()
