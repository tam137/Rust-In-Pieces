import json
import os
import random
import subprocess
import time
import csv
import argparse
import concurrent.futures
import threading

class SPSATuner:
    def __init__(self, params_file, state_file, history_file, engine_path, mm_path, games_per_iter=250, workers=8, time_ms=2000, inc_ms=100, mutate_pct=3.0, lr=2.0, logpath=""):
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
        
        # SPSA Constants (Standard values)
        self.a = lr     # Base learning rate (depends on gradient magnitude)
        self.c = 3.0     # Base perturbation size
        self.A = 100.0   # 10% of total iterations
        self.alpha = 0.602
        self.gamma = 0.101

        with open(self.params_file, "r") as f:
            self.param_defs = json.load(f)
            
        self.param_names = list(self.param_defs.keys())
        
        self.k = 1
        self.theta = {k: float(v["value"]) for k, v in self.param_defs.items()}
        
        # Load state if exists
        if os.path.exists(self.state_file):
            with open(self.state_file, "r") as f:
                state = json.load(f)
                self.k = state["k"]
                self.theta = state["theta"]
                print(f"Loaded state from iteration {self.k}")
        else:
            # Initialize history file
            with open(self.history_file, "w", newline="") as f:
                writer = csv.writer(f)
                writer.writerow(["Iteration", "Score"] + self.param_names)

    def _get_a_k(self):
        return self.a / ((self.A + self.k) ** self.alpha)

    def _get_c_k(self):
        return self.c / (self.k ** self.gamma)

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
        a_k = self._get_a_k()
        
        # Bernoulli +-1
        delta = {k: random.choice([-1, 1]) for k in self.param_names}
        
        # Calculate integer perturbation steps based on percentage
        step_sizes = {}
        for k in self.param_names:
            base_val = abs(self.theta[k])
            step = max(1.0, round(base_val * (self.mutate_pct / 100.0)))
            step_sizes[k] = step

        theta_plus = {k: self.theta[k] + step_sizes[k] * delta[k] for k in self.param_names}
        theta_minus = {k: self.theta[k] - step_sizes[k] * delta[k] for k in self.param_names}
        
        score = self.run_match_batch(theta_plus, theta_minus)
        
        # Gradient estimation
        diff = 2.0 * score - 1.0
        
        for k in self.param_names:
            g_k = diff / (2.0 * step_sizes[k] * delta[k])
            # Scale learning rate by the parameter's magnitude to prevent throttling
            param_scale = max(1.0, abs(self.param_defs[k]["value"]))
            a_k_scaled = a_k * param_scale
            self.theta[k] += a_k_scaled * g_k
            
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
            row = [self.k - 1, score] + [self.theta[k] for k in self.param_names]
            writer.writerow(row)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", required=True)
    parser.add_argument("--mm", required=True)
    parser.add_argument("--games", type=int, default=250)
    parser.add_argument("--workers", type=int, default=8, help="Number of parallel games to run simultaneously")
    parser.add_argument("--time", type=int, default=2, help="Time per game in seconds")
    parser.add_argument("--inc", type=int, default=100, help="Increment per move in milliseconds")
    parser.add_argument("--mutate", type=float, default=3.0, help="Perturbation percentage per parameter (e.g., 3 for 3%)")
    parser.add_argument("--lr", type=float, default=2.0, help="Base learning rate (a)")
    parser.add_argument("--logpath", default="/root/mattmagie/tuning/enginelogs")
    args = parser.parse_args()
    
    tuner = SPSATuner(
        params_file="parameters.json",
        state_file="spsa_state.json",
        history_file="spsa_history.csv",
        engine_path=args.engine,
        mm_path=args.mm,
        games_per_iter=args.games,
        workers=args.workers,
        time_ms=args.time * 1000,
        inc_ms=args.inc,
        mutate_pct=args.mutate,
        lr=args.lr,
        logpath=args.logpath
    )
    
    # Run 100 iterations as a test
    for _ in range(100):
        tuner.step()
