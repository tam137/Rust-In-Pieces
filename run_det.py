import pexpect
import time

for i in range(5):
    child = pexpect.spawn('./target/release/suprah', encoding='utf-8')
    child.sendline('position fen r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/2N2N2/PPPP1PPP/R1BQKB1R w KQkq - 4 4')
    child.sendline('go depth 6')
    child.expect('bestmove (\\S+)')
    print(f"Run {i}: {child.match.group(1)}")
    child.close()
