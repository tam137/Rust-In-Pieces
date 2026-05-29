import subprocess
import time

p = subprocess.Popen(["cargo", "run", "--release"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
p.stdin.write("uci\nisready\nposition startpos\ngo depth 9\n")
p.stdin.flush()

while True:
    line = p.stdout.readline()
    if not line:
        break
    print(line.strip())
    if "bestmove" in line:
        break

p.stdin.write("quit\n")
p.stdin.flush()
p.wait()
