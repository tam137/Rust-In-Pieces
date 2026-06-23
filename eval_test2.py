import pexpect

def test_fen(fen, version):
    child = pexpect.spawn('./target/release/suprah', encoding='utf-8')
    child.sendline(f'position fen {fen}')
    child.sendline('go depth 8')
    child.expect('bestmove (\\S+)')
    output = child.before
    child.close()
    return output

fens = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r1bqk2r/pp2bppp/2n1pn2/2pp2B1/3P4/2N1PN2/PPP1BPPP/R2QK2R w KQkq - 2 7",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"
]

import os
import re

for fen in fens:
    os.system("git checkout v0.17.0 >/dev/null 2>&1 && cargo build --release >/dev/null 2>&1")
    out1 = test_fen(fen, 'v0.17.0')
    m1 = re.search(r'depth 8 score cp (-?\\d+) .* nodes (\\d+)', out1)
    
    os.system("git checkout v0.17.1 >/dev/null 2>&1 && cargo build --release >/dev/null 2>&1")
    out2 = test_fen(fen, 'v0.17.1')
    m2 = re.search(r'depth 8 score cp (-?\\d+) .* nodes (\\d+)', out2)
    
    print(f"FEN: {fen}")
    if m1 and m2:
        print(f"  v0.17.0: cp {m1.group(1)} nodes {m1.group(2)}")
        print(f"  v0.17.1: cp {m2.group(1)} nodes {m2.group(2)}")
        if m1.group(1) != m2.group(1) or m1.group(2) != m2.group(2):
            print("  MISMATCH!")
    else:
        print("  Regex failed!")
