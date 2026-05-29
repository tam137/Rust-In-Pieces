# LCT II Test Results: Version v0.11.5

* **Engine Binary**: `target/release/suprah`
* **Positions Solved**: 6 / 35 (17.1%)
* **Total Points**: 180 / 1050
* **Estimated ELO**: **2080 ELO** (Expanded Opening Book & Playable Exotic Lines)

---

# LCT II Test Results: Version v0.11.2

## Historical Comparison

| Version | ELO | Total Points | Solved | Positional | Tactical | Endgame |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `v0.11.5` (Expanded Opening Book) | **2080** | 180 | 6/35 | 2/14 | 2/12 | 2/9 |
| `v0.11.2` (King Safety & Threat Matrix) | **2110** | 210 | 7/35 | 2/14 | 2/12 | 3/9 |
| `v0.11.0` (O(1) Incremental Evaluation) | **2140** | 240 | 8/35 | 3/14 | 2/12 | 3/9 |
| `v0.10.12` (PST, Phase Calc, King Danger) | **2080** | 180 | 6/35 | 2/14 | 2/12 | 2/9 |
| `v0.10.11` (Pawn eval changes) | **2020** | 120 | 4/35 | 2/14 | 1/12 | 1/9 |
| `v0.10.8` (Dynamic UCI Hash Option) | **2105** | 205 | 7/35 | 3/14 | 2/12 | 2/9 |
| `v0.10.7` (UCI Move Overhead) | **2105** | 205 | 7/35 | 3/14 | 2/12 | 2/9 |
| `v0.10.6` (Lazy SEE & Fast Masks) | **2105** | 205 | 7/35 | 3/14 | 2/12 | 2/9 |
| `v0.10.5` (Bishop bugfix & Tempo de-escalation) | **2080** | 180 | 6/35 | 2/14 | 2/12 | 2/9 |
| `v0.10.4` (Piece-Based Dynamic Positional Cap) | **2200** | 300 | 10/35 | 2/14 | 6/12 | 2/9 |
| `v0.10.3` (HCE Positional Cap & 5x Damping Tuning) | **2110** | 210 | 7/35 | 2/14 | 3/12 | 2/9 |
| `v0.10.0` (Lock-Free TT, SEE Ordering, Aggressiveness) | **2225** | 325 | 11/35 | 2/14 | 6/12 | 3/9 |
| `v0.9.6` (Time Checks & NMP Recursion Fix) | **2200** | 300 | 10/35 | 2/14 | 5/12 | 3/9 |
| `v0.9.4` (Dynamic NMP & Verification Search) | **2200** | 300 | 10/35 | 2/14 | 5/12 | 3/9 |
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

### Scoreboard by Category (v0.11.5)

* **Positional**: 2 / 14 solved (14.3%) | 60 points
* **Tactical**: 2 / 12 solved (16.7%) | 60 points
* **Endgame**: 2 / 9 solved (22.2%) | 60 points

---

## Detailed Results (v0.11.5)

| ID | Category | Description | Correct Move | Engine Move | Solved? | Time | Points |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| LCTII.POS.01 | Positional | Chernin - Miles, Tunis 1985 | d5d6 | g3g4 | **NO** | - | 0 |
| LCTII.POS.02 | Positional | Lilienthal - Botvinnik, Moskau 1945 | d6b4 | d6b4 | **YES** | 0.01s | 30 |
| LCTII.POS.03 | Positional | Boissel - Boulard, corr. 1994 | f2c5 | f2h4 | **NO** | - | 0 |
| LCTII.POS.04 | Positional | Kaplan - Kopec, USA 1975 | e6e5 | b8c6 | **NO** | - | 0 |
| LCTII.POS.05 | Positional | Estrin - Pytel, Albena 1973 | d7b5 | b7b5 | **NO** | - | 0 |
| LCTII.POS.06 | Positional | Nimzowitsch - Marshall 1927 | e6e5 | d8d7 | **NO** | - | 0 |
| LCTII.POS.07 | Positional | Alehine - Nimzowitsch, Semmering 1926 | c3d1 | c3a4 | **NO** | - | 0 |
| LCTII.POS.08 | Positional | Unzicker - Fischer, Varna 1962 | g2h3 | a1d1 | **NO** | - | 0 |
| LCTII.POS.09 | Positional | Boissel - Del Gobbo, corr. 1994 | a7d4 | f4d5 | **NO** | - | 0 |
| LCTII.POS.10 | Positional | A.Sokolov - Salov, Leningrad 1987 | e7f8 | e7d8 | **NO** | - | 0 |
| LCTII.POS.11 | Positional | Capablanca - Ragozin, Moskau 1935 | h3h4 | c1a3 | **NO** | - | 0 |
| LCTII.POS.12 | Positional | Zuckerman - Evans, USA 1967 | c6b6 | d6b4 | **NO** | - | 0 |
| LCTII.POS.13 | Positional | Capablanca - Ragozin, Moskau 1935 | c3a2 | c3a2 | **YES** | 0.93s | 30 |
| LCTII.POS.14 | Positional | Polugaevsky - Nezhmetdinov, Sochi 1958 | d4d5 | c1g5 | **NO** | - | 0 |
| LCTII.TAC.01 | Tactical | Zubarev - Geller, USSR 1950 | c4d6 | h4h1 | **NO** | - | 0 |
| LCTII.TAC.02 | Tactical | Keres - Eliskases, Noordwijk 1938 | h5h7 | h5h7 | **YES** | 0.18s | 30 |
| LCTII.TAC.03 | Tactical | Drimer - Rellstab, corr. 1968 | f6f3 | e8c8 | **NO** | - | 0 |
| LCTII.TAC.04 | Tactical | Hort - Wade, Pajulahti 1974 | e5f6 | e5f6 | **YES** | 7.28s | 30 |
| LCTII.TAC.05 | Tactical | Fischer - Myagmarsuren, Sousse 1967 | h3h7 | b2c1 | **NO** | - | 0 |
| LCTII.TAC.06 | Tactical | R.Byrne - Fischer, New York 1963 | e5f6 | c2e4 | **NO** | - | 0 |
| LCTII.TAC.07 | Tactical | Wojtkiewicz - Kasparov, Simultan 1993 | c8c3 | b7e4 | **NO** | - | 0 |
| LCTII.TAC.08 | Tactical | Nei - Bronstein, Moskau 1963 | d5f6 | d1d3 | **NO** | - | 0 |
| LCTII.TAC.09 | Tactical | Stein - Birbrager, USSR 1966 | a2d2 | h5f4 | **NO** | - | 0 |
| LCTII.TAC.10 | Tactical | Fischer - Gadia, Simultan 1965 | f4h6 | g1f1 | **NO** | - | 0 |
| LCTII.TAC.11 | Tactical | Nezhmetdinov - Tal, Baku 1961 | g5h7 | c1f4 | **NO** | - | 0 |
| LCTII.TAC.12 | Tactical | Vaganyan - Kupreichik, USSR 1980 | e4e5 | f1f2 | **NO** | - | 0 |
| LCTII.END.01 | Endgame | Pawn Endgame Study | f5f6 | c2b2 | **NO** | - | 0 |
| LCTII.END.02 | Endgame | Rook Endgame Study | f4f5 | f4f5 | **YES** | 0.10s | 30 |
| LCTII.END.03 | Endgame | Bishop Endgame Study | c6e4 | c6e4 | **YES** | 6.79s | 30 |
| LCTII.END.04 | Endgame | Rook and Pawn Study | h4h3 | c4b4 | **NO** | - | 0 |
| LCTII.END.05 | Endgame | Endgame Combination Study | a5a6 | e1d1 | **NO** | - | 0 |
| LCTII.END.06 | Endgame | Knight and Bishop Study | f5f4 | e5g4 | **NO** | - | 0 |
| LCTII.END.07 | Endgame | Endgame Rook Slide Study | d2b4 | g1g2 | **NO** | - | 0 |
| LCTII.END.08 | Endgame | Positional Pawn Breakthrough Study | c4c5 | f2e3 | **NO** | - | 0 |
| LCTII.END.09 | Endgame | King and Bishop Endgame Study | f3g4 | f4g5 | **NO** | - | 0 |
