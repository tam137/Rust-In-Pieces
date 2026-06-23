import pexpect

child = pexpect.spawn('./target/release/suprah', encoding='utf-8')
child.sendline('position fen r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/2N2N2/PPPP1PPP/R1BQKB1R w KQkq - 4 4')
child.sendline('go depth 12')
child.expect('bestmove (\\S+)')
print(child.before)
child.close()
