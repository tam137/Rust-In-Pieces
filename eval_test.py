import pexpect

def get_eval(version):
    child = pexpect.spawn('./target/release/suprah', encoding='utf-8')
    child.sendline('position fen r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/2N2N2/PPPP1PPP/R1BQKB1R w KQkq - 4 4')
    child.sendline('go depth 12')
    child.expect('score cp (-?\\d+)')
    score = child.match.group(1)
    child.expect('bestmove (\\S+)')
    move = child.match.group(1)
    child.close()
    return score, move

import os
os.system("git checkout v0.17.0 && cargo build --release")
score1, move1 = get_eval('v0.17.0')
os.system("git checkout v0.17.1 && cargo build --release")
score2, move2 = get_eval('v0.17.1')

print(f"v0.17.0 eval: {score1}, move: {move1}")
print(f"v0.17.1 eval: {score2}, move: {move2}")
