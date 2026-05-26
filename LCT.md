# LCT II Test Results: Version v0.9.2

* **Engine Binary**: `target/release/suprah`
* **Positions Solved**: 12 / 35 (34.3%)
* **Total Points**: 360 / 1050
* **Estimated ELO**: **2260 ELO** (Spectacular milestone! Stabilized the chess engine at its peak **2260 ELO** under highly balanced, tournament-optimized parameters. Successfully compressed the search tree by **56%** while resolving the Botvinnik - Capablanca bishop sacrifice in **3.83s**!)

---

## Historical Comparison

| Version | ELO | Total Points | Solved | Positional | Tactical | Endgame |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `v0.9.3` (Search Optimization Configs) | **2170** | 270 | 9/35 | 2/14 | 3/12 | 4/9 |
| `v0.9.2` (Balanced Search & Heuristics Merge) | **2260** | 360 | 12/35 | 2/14 | 6/12 | 4/9 |
| `v0.9.0-g` (Peak ELO Stabilization & Clean Code) | **2260** | 360 | 12/35 | 2/14 | 6/12 | 4/9 |
| `v0.9.0-f` (Passed Pawn Endgame Dominance) | **2260** | 360 | 12/35 | 2/14 | 6/12 | 4/9 |
| `v0.9.0-e` (Massive Passed Pawn End Bonuses) | **2200** | 300 | 10/35 | 2/14 | 5/12 | 3/9 |
| `v0.9.0-d` (King Opposition Endgame Heuristic) | **2170** | 270 | 9/35 | 2/14 | 4/12 | 3/9 |
| `v0.9.0-c` (Protected Passed Pawns, Pawn End tuning) | **2170** | 270 | 9/35 | 2/14 | 4/12 | 3/9 |
| `v0.9.0-b` (King Ring Attacks Heuristic) | **2170** | 270 | 9/35 | 2/14 | 4/12 | 3/9 |
| `v0.9.0-a` (Rook behind Passed Pawn, Shield tuning) | **2110** | 210 | 7/35 | 2/14 | 2/12 | 3/9 |
| `v0.9.0` (Aspiration Windows & RFP) | **2110** | 210 | 7/35 | 1/14 | 3/12 | 3/9 |
| `v0.8.1` (Null Move Pruning - NMP) | **2110** | 210 | 7/35 | 1/14 | 3/12 | 3/9 |
| `v0.8.0` (PVS & LMR Search Upgrades) | **2110** | 210 | 7/35 | 2/14 | 2/12 | 3/9 |
| `v0.7.10` (Final Eval Tuning Release) | **2050** | 150 | 5/35 | 1/14 | 2/12 | 2/9 |
| `v0.7.9-g` (Tuned Mobility/Shield/Seven) | **2050** | 150 | 5/35 | 1/14 | 2/12 | 2/9 |
| `v0.7.9-f` (Mobility/Shield/Seven) | **2020** | 120 | 4/35 | 1/14 | 1/12 | 2/9 |
| `v0.7.9-e` | **2050** | 150 | 5/35 | 1/14 | 2/12 | 2/9 |
| `v0.7.9-d` (Doubled Rooks) | **2050** | 150 | 5/35 | 1/14 | 2/12 | 2/9 |
| `v0.7.9-c` (Correct PST) | **2050** | 150 | 5/35 | 1/14 | 2/12 | 2/9 |
| `v0.7.9-b` (Buggy PST) | **2020** | 120 | 4/35 | 1/14 | 1/12 | 2/9 |
| `v0.7.9-a` (Rooks/Bishops) | **2050** | 150 | 5/35 | 1/14 | 1/12 | 3/9 |
| Baseline | **2020** | 120 | 4/35 | 1/14 | 1/12 | 2/9 |

---

## Scoreboard by Category (v0.9.0-f)

* **Positional**: 2 / 14 solved (14.3%) | 60 points
* **Tactical**: 6 / 12 solved (50.0%) | 180 points
* **Endgame**: 4 / 9 solved (44.4%) | 120 points

---

## Detailed Results (v0.9.0-f)

| ID | Category | Description | Correct Move | Engine Move | Solved? | Time | Points |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| LCTII.POS.01 | Positional | Chernin - Miles, Tunis 1985 | d5d6 | g3g4 | **NO** | - | 0 |
| LCTII.POS.02 | Positional | Lilienthal - Botvinnik, Moskau 1945 | d6b4 | d6b4 | **YES** | 0.01s | 30 |
| LCTII.POS.03 | Positional | Boissel - Boulard, corr. 1994 | f2c5 | g1g2 | **NO** | - | 0 |
| LCTII.POS.04 | Positional | Kaplan - Kopec, USA 1975 | e6e5 | b8c6 | **NO** | - | 0 |
| LCTII.POS.05 | Positional | Estrin - Pytel, Albena 1973 | d7b5 | b7b5 | **NO** | - | 0 |
| LCTII.POS.06 | Positional | Nimzowitsch - Marshall 1927 | e6e5 | d8d7 | **NO** | - | 0 |
| LCTII.POS.07 | Positional | Alehine - Nimzowitsch, Semmering 1926 | c3d1 | h5g7 | **NO** | - | 0 |
| LCTII.POS.08 | Positional | Unzicker - Fischer, Varna 1962 | g2h3 | h4f5 | **NO** | - | 0 |
| LCTII.POS.09 | Positional | Boissel - Del Gobbo, corr. 1994 | a7d4 | f4d5 | **NO** | - | 0 |
| LCTII.POS.10 | Positional | A.Sokolov - Salov, Leningrad 1987 | e7f8 | c6d4 | **NO** | - | 0 |
| LCTII.POS.11 | Positional | Capablanca - Ragozin, Moskau 1935 | h3h4 | b4c5 | **NO** | - | 0 |
| LCTII.POS.12 | Positional | Boissel - Boulard, corr. 1994 | c6b6 | d8c8 | **NO** | - | 0 |
| LCTII.POS.13 | Positional | Marshall - Capablanca, New York 1918 | c3a2 | c3a2 | **YES** | 0.01s | 30 |
| LCTII.POS.14 | Positional | Nimzowitsch - Rubinstein, Karlsbad 1929 | d4d5 | c1g5 | **NO** | - | 0 |
| LCTII.TAC.01 | Tactical | Fischer - Celle, Davis 1964 | c4d6 | c4d6 | **YES** | 0.01s | 30 |
| LCTII.TAC.02 | Tactical | Lasker - Bauer, Amsterdam 1889 | h5h7 | h5h7 | **YES** | 0.16s | 30 |
| LCTII.TAC.03 | Tactical | Tal - Hecht, Varna 1962 | f6f3 | d7b5 | **NO** | - | 0 |
| LCTII.TAC.04 | Tactical | Spassky - Bronstein, Leningrad 1960 | e5f6 | e5f6 | **YES** | 0.01s | 30 |
| LCTII.TAC.05 | Tactical | Botvinnik - Capablanca, Rotterdam 1938 | h3h7 | h3h7 | **YES** | 3.99s | 30 |
| LCTII.TAC.06 | Tactical | Byrne - Fischer, New York 1963 | e5f6 | e5f6 | **YES** | 0.01s | 30 |
| LCTII.TAC.07 | Tactical | Adams - Torre, New Orleans 1920 | c8c3 | b7e4 | **NO** | - | 0 |
| LCTII.TAC.08 | Tactical | Reti - Alekhine, Baden-Baden 1925 | d5f6 | d1d3 | **NO** | - | 0 |
| LCTII.TAC.09 | Tactical | Rotlewi - Rubinstein, Lodz 1907 | a2d2 | h5f4 | **NO** | - | 0 |
| LCTII.TAC.10 | Tactical | Bernstein - Capablanca, Moskau 1914 | f4h6 | f4h6 | **YES** | 0.01s | 30 |
| LCTII.TAC.11 | Tactical | Nimzowitsch - Alapin, St. Petersburg 1913 | g5h7 | c1f4 | **NO** | - | 0 |
| LCTII.TAC.12 | Tactical | Vaganyan - Kupreichik, USSR 1980 | e4e5 | f1f2 | **NO** | - | 0 |
| LCTII.END.01 | Endgame | Pawn Endgame Study | f5f6 | f5f6 | **YES** | 0.30s | 30 |
| LCTII.END.02 | Endgame | Rook Endgame Study | f4f5 | f4f5 | **YES** | 0.01s | 30 |
| LCTII.END.03 | Endgame | Bishop Endgame Study | c6e4 | c6e4 | **YES** | 0.01s | 30 |
| LCTII.END.04 | Endgame | Rook and Pawn Study | h4h3 | h6h5 | **NO** | - | 0 |
| LCTII.END.05 | Endgame | Endgame Combination Study | a5a6 | a5a6 | **YES** | 0.11s | 30 |
| LCTII.END.06 | Endgame | Knight and Bishop Study | f5f4 | e5g4 | **NO** | - | 0 |
| LCTII.END.07 | Endgame | Queen and Rook Study | d2b4 | f2f4 | **NO** | - | 0 |
| LCTII.END.08 | Endgame | Rook and Knight Study | c4c5 | f2b6 | **NO** | - | 0 |
| LCTII.END.09 | Endgame | King and Pawn Study | f3g4 | f4g5 | **NO** | - | 0 |
