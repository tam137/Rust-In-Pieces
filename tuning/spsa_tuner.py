import json
import os
import random
import subprocess
import time
import csv
import argparse
import concurrent.futures

class SPSATuner:
    def __init__(self, params_file, state_file, history_file, engine_path, mm_path, games_per_iter=250):
        self.params_file = params_file
        self.state_file = state_file
        self.history_file = history_file
        self.engine_path = engine_path
        self.mm_path = mm_path
        self.games_per_iter = games_per_iter
        
        # SPSA Constants (Standard values)
        self.a = 2.0     # Base learning rate (depends on gradient magnitude)
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
            tmp_pgn = f"tuning/tmp_{self.k}_{i}.pgn"
            cmd = [
                self.mm_path,
                self.engine_path,
                self.engine_path,
                "/dev/null",
                tmp_pgn,
                "SPSA_Tuning",
                "Local",
                str(i),
                "2000",   # 2 seconds
                "100",    # 100 ms
                "log_off",
                "debug_off",
                opts_plus,
                opts_minus
            ]
            subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            return tmp_pgn

        wins = 0
        losses = 0
        draws = 0
        
        # Run in parallel
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as executor:
            pgn_files = list(executor.map(run_single_game, range(self.games_per_iter)))
            
        # Aggregate results
        for pf in pgn_files:
            if os.path.exists(pf):
                with open(pf, "r") as f:
                    content = f.read()
                    if "[Result \"1-0\"]" in content:
                        wins += 1
                    elif "[Result \"0-1\"]" in content:
                        losses += 1
                    elif "[Result \"1/2-1/2\"]" in content:
                        draws += 1
                os.remove(pf)
                
        print(f"Results: +{wins} ={draws} -{losses}")
        total = wins + draws + losses
        if total == 0:
            return 0.5
            
        return (wins + 0.5 * draws) / total

    def step(self):
        c_k = self._get_c_k()
        a_k = self._get_a_k()
        
        # Bernoulli +-1
        delta = {k: random.choice([-1, 1]) for k in self.param_names}
        
        theta_plus = {k: self.theta[k] + c_k * delta[k] for k in self.param_names}
        theta_minus = {k: self.theta[k] - c_k * delta[k] for k in self.param_names}
        
        score = self.run_match_batch(theta_plus, theta_minus)
        
        # Gradient estimation: (L(theta_plus) - L(theta_minus)) / (2 * c_k * delta_k)
        # We can map score [0, 1] to E_plus. E_minus = 1 - score.
        # Diff = score - (1 - score) = 2*score - 1
        diff = 2.0 * score - 1.0
        
        for k in self.param_names:
            g_k = diff / (2.0 * c_k * delta[k])
            self.theta[k] += a_k * g_k
            
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
    args = parser.parse_args()
    
    tuner = SPSATuner(
        params_file="tuning/parameters.json",
        state_file="tuning/spsa_state.json",
        history_file="tuning/spsa_history.csv",
        engine_path=args.engine,
        mm_path=args.mm,
        games_per_iter=args.games
    )
    
    # Run 100 iterations as a test
    for _ in range(100):
        tuner.step()
