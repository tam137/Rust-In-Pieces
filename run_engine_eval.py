import subprocess

p = subprocess.Popen(['./target/release/suprah'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

p.stdin.write("uci\n")
p.stdin.write("position fen r1b2rk1/p1p1R1pp/2p5/8/2Q3b1/6Pq/PPP4P/R1B3K1 b - - 2 18\n")
p.stdin.write("eval\n")
p.stdin.write("quit\n")
p.stdin.flush()

for line in p.stdout:
    print(line.strip())

p.wait()
