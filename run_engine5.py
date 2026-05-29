import subprocess

p = subprocess.Popen(['./target/release/suprah'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

p.stdin.write("uci\n")
p.stdin.write("position fen r1b1k2r/ppp1b1pp/2p5/4N3/3PR3/6Pq/PPP4P/R1BQ2K1 w kq - 1 15 moves d1h5\n")
p.stdin.write("go depth 9\n")
p.stdin.flush()

for line in p.stdout:
    print(line.strip())
    if 'bestmove' in line:
        break

p.stdin.write("quit\n")
p.stdin.flush()
p.wait()
