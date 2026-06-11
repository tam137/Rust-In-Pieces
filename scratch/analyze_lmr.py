import math

def get_table(divisor_val):
    divisor = divisor_val / 100.0
    table = [[0 for _ in range(64)] for _ in range(64)]
    for depth in range(1, 64):
        for move_idx in range(1, 64):
            d = float(depth)
            m = float(move_idx)
            val = int(math.log(d) * math.log(m) / divisor)
            table[depth][move_idx] = max(0, val)
    return table

def compare(v1, v2):
    t1 = get_table(v1)
    t2 = get_table(v2)
    diff = 0
    total = 63 * 63
    for d in range(1, 64):
        for m in range(1, 64):
            if t1[d][m] != t2[d][m]:
                diff += 1
    return diff

print(f"Diff 195 vs 196: {compare(195, 196)} cells out of 3969 ({compare(195, 196)/3969*100:.2f}%)")
print(f"Diff 189 vs 202: {compare(189, 202)} cells out of 3969 ({compare(189, 202)/3969*100:.2f}%)")
print(f"Diff 195 vs 190 (baseline vs perturbed -): {compare(195, 190)} cells out of 3969 ({compare(195, 190)/3969*100:.2f}%)")
print(f"Diff 195 vs 202 (baseline vs perturbed +): {compare(195, 202)} cells out of 3969 ({compare(195, 202)/3969*100:.2f}%)")
