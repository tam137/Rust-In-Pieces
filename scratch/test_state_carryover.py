import pexpect
import time

def run_test():
    # Start engine
    child = pexpect.spawn('./target/release/suprah', encoding='utf-8')
    child.logfile = open('scratch/carryover_engine.log', 'w')
    
    # Game 1: search a position
    print("--- Game 1 ---")
    child.sendline('ucinewgame')
    child.sendline('position fen r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/2N2N2/PPPP1PPP/R1BQKB1R w KQkq - 4 4')
    child.sendline('go depth 8')
    child.expect('bestmove (\\S+)')
    out1 = child.before
    print(out1)
    
    # Make some other moves/searches to populate transposition and pawn tables
    child.sendline('position fen r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1')
    child.sendline('go depth 8')
    child.expect('bestmove (\\S+)')
    
    # Game 2: Send ucinewgame, then search the Game 1 position again
    print("--- Game 2 ---")
    child.sendline('ucinewgame')
    child.sendline('position fen r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/2N2N2/PPPP1PPP/R1BQKB1R w KQkq - 4 4')
    child.sendline('go depth 8')
    child.expect('bestmove (\\S+)')
    out2 = child.before
    print(out2)
    
    child.close()

if __name__ == '__main__':
    run_test()
