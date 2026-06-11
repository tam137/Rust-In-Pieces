import os
import subprocess
import re
import shutil

src_dir = "/tmp/build_bench/v8"
backup_files = {
    "src/config.rs": os.path.join(src_dir, "src/config.rs"),
    "src/game_handler.rs": os.path.join(src_dir, "src/game_handler.rs"),
    "src/threads.rs": os.path.join(src_dir, "src/threads.rs"),
}

# Create backups
for rel_path, abs_path in backup_files.items():
    shutil.copyfile(abs_path, abs_path + ".bak")

def restore_all():
    for rel_path, abs_path in backup_files.items():
        shutil.copyfile(abs_path + ".bak", abs_path)

def compile_and_run():
    # Build
    res = subprocess.run(
        ["source $HOME/.cargo/env && cargo build --release"],
        shell=True, cwd=src_dir, capture_output=True, text=True, executable="/bin/bash"
    )
    if res.returncode != 0:
        print("Compilation failed!")
        print(res.stderr)
        return None
        
    # Run benchmark
    # We run (echo "test"; sleep 5) | target/release/suprah
    try:
        run_res = subprocess.run(
            ["(echo 'test'; sleep 5) | ./target/release/suprah"],
            shell=True, cwd=src_dir, capture_output=True, text=True, timeout=10, executable="/bin/bash"
        )
        output = run_res.stdout
    except subprocess.TimeoutExpired:
        print("Run timed out!")
        return None
        
    # Extract search time
    # Time taken [service.search.get_moves(&mut service.fen.set_fen(mid_game_fen), 4, false, ...)]: 16.113519ms
    match = re.search(r"Time taken \[service\.search\.get_moves.*false.*\]:\s*([\d\.]+)ms", output)
    if match:
        return float(match.group(1))
    else:
        print("Could not parse search time from output!")
        print(output)
        return None

results = {}

# Restore first
restore_all()

# 1. Benchmark Variation A: Unmodified v0.13.8
print("Benchmarking Variation A (Unmodified v0.13.8)...")
t_a = compile_and_run()
results["A (Unmodified v0.13.8)"] = t_a
print(f"Time: {t_a} ms")

# 2. Benchmark Variation B: log_all_parameters commented out in game_handler.rs
print("\nBenchmarking Variation B (log_all_parameters calls commented out)...")
restore_all()
with open(backup_files["src/game_handler.rs"], "r") as f:
    code = f.read()

# Comment out active_config.log_all_parameters(&logger);
modified_code = code.replace("active_config.log_all_parameters(&logger);", "// active_config.log_all_parameters(&logger);")
with open(backup_files["src/game_handler.rs"], "w") as f:
    f.write(modified_code)

t_b = compile_and_run()
results["B (log_all_parameters commented out)"] = t_b
print(f"Time: {t_b} ms")

# 3. Benchmark Variation C: log_path & log_all_parameters completely removed (equivalent to v0.13.4)
print("\nBenchmarking Variation C (log_path and logging completely removed)...")
restore_all()

# Modify config.rs
with open(backup_files["src/config.rs"], "r") as f:
    config_code = f.read()
# Remove pub log_path: String,
config_code = config_code.replace("pub log_path: String,", "")
config_code = config_code.replace("log_path: String::new(),", "")
# Remove log_all_parameters function definition
# We can find pub fn log_all_parameters and remove everything until the end of impl Config
impl_start = config_code.find("pub fn log_all_parameters")
if impl_start != -1:
    config_code = config_code[:impl_start] + "}"

with open(backup_files["src/config.rs"], "w") as f:
    f.write(config_code)

# Modify game_handler.rs
with open(backup_files["src/game_handler.rs"], "r") as f:
    gh_code = f.read()
gh_code = gh_code.replace("active_config.log_all_parameters(&logger);", "")
gh_code = gh_code.replace('"logpath" | "log_path" => { active_config.log_path = val_str.clone(); },', "")
with open(backup_files["src/game_handler.rs"], "w") as f:
    f.write(gh_code)

# Modify threads.rs (to restore to v0.13.4 version, or at least remove option declarations/logging)
with open(backup_files["src/threads.rs"], "r") as f:
    threads_code = f.read()
threads_code = threads_code.replace('stdout.write("option name LogPath type string default <empty>");', "")
# We do not need to fully restore threads.rs since the option processing only runs on setoption,
# but let's make sure it doesn't try to use log_path/logger
with open(backup_files["src/threads.rs"], "w") as f:
    f.write(threads_code)

t_c = compile_and_run()
results["C (logging code completely removed)"] = t_c
print(f"Time: {t_c} ms")

# Cleanup and restore
restore_all()

print("\n==========================================")
print("Isolation Results:")
print("==========================================")
for var, t in results.items():
    if t:
        print(f"{var:<40}: {t:.6f} ms")
    else:
        print(f"{var:<40}: Failed")
