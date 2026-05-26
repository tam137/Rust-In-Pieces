# Suchbaum-Optimierungen für SupraH (MinMaxOptimizeTask)
Dieses Dokument beschreibt vier fortgeschrittene Such- und Zugordnungsoptimierungen, basierend auf bewährten Konzepten moderner Schach-Engines (wie Stockfish, Ethereal und SleepMind). 

Die Implementierung dieser Features in SupraH soll die Suchtiefe signifikant erhöhen, die Ruhesuche (Q-Search) fokussieren und die Taktiksicherheit im tiefen Suchbaum stärken.

---

## Inhaltsverzeichnis
1. [Optimierung 1: SEE & Delta Pruning in der Ruhesuche](#optimierung-1-see--delta-pruning-in-der-ruhesuche)
2. [Optimierung 2: Dynamische LMR mit logarithmischer Tabelle & History-Koppelung](#optimierung-2-dynamische-lmr-mit-logarithmischer-tabelle--history-koppelung)
3. [Optimierung 3: Counter-Moves Heuristic & History-Malus in der Zugordnung](#optimierung-3-counter-moves-heuristic--history-malus-in-der-zugordnung)
4. [Optimierung 4: Dynamisches NMP mit Verifikationssuche](#optimierung-4-dynamisches-nmp-mit-verifikationssuche)

---

## Optimierung 1: SEE & Delta Pruning in der Ruhesuche

### 1. Konzeptionelle Grundlagen
Die Ruhesuche (Quiescence Search) berechnet alle Schlagzüge, um taktische Einstürze am Ende des regulären Suchbaums zu verhindern.
*   **Problem:** SupraH generiert *alle* Schlagzüge und sucht sie ab. Das führt dazu, dass taktisch katastrophale Züge (z. B. Dame schlägt gedeckten Bauern) voll berechnet werden, was wertvolle Suchknoten verschwendet.
*   **Lösung:** 
    1.  **Delta Pruning:** Bevor ein Schlagzug ausgeführt wird, prüfen wir, ob das Schlagen des Materials überhaupt das Potenzial hat, das aktuelle Alpha anzuheben. Wenn `stand_pat + Materialwert(Opfer) + Delta-Marge < Alpha`, brechen wir ab.
    2.  **Static Exchange Evaluation (SEE):** Wir simulieren rekursiv alle Schläge auf dem Zielfeld (ohne den eigentlichen Suchbaum zu verändern), um festzustellen, ob der Abtausch für die schlagende Seite positiv oder ausgeglichen ist. Verliert der Abtausch Material, wird der Zug in der Ruhesuche gar nicht erst generiert oder sofort verworfen.

### 2. Mathematische Grundlagen & Algorithmus
SEE baut eine "Angreifer-Kette" auf dem Zielfeld auf. Es ermittelt abwechselnd für beide Seiten die jeweils wertniedrigste angreifende Figur (Pawn $\rightarrow$ Knight $\rightarrow$ Bishop $\rightarrow$ Rook $\rightarrow$ Queen $\rightarrow$ King) und simuliert das Schlagen auf dem Zielfeld, bis keine Angreifer mehr vorhanden sind.
Daraus ergibt sich ein Minimax-Score-Array der Abtauschstufen:

$$\text{Score}[d] = \text{PieceValue}(d) - \text{Score}[d+1]$$

### 3. Rust-Entwurf (Pseudocode)

```rust
// In src/search_service.rs oder einem neuen Modul src/see.rs

impl SearchService {
    /// Gibt an, ob der Abtausch auf dem Zielfeld von `mv` mindestens den `threshold` (Grenzwert) einbringt.
    pub fn see_ge(&self, board: &Board, mv: &Turn, threshold: i16) -> bool {
        let from = mv.from as usize;
        let to = mv.to as usize;
        
        // Einfache Abschätzung: Wenn der direkte Materialwert des geschlagenen Stücks 
        // abzüglich des schlagenden Stücks bereits >= threshold ist, ist der Zug meist gut.
        let victim_val = self.get_piece_value(board.get_piece_at(to as u8));
        let attacker_val = self.get_piece_value(board.get_piece_at(from as u8));
        
        let mut swap = victim_val - threshold;
        if swap < 0 {
            return false; // Sogar das erste Schlagen reicht nicht aus
        }
        
        swap -= attacker_val;
        if swap >= 0 {
            return true; // Selbst wenn wir zurückgeschlagen werden, gewinnen wir Material
        }
        
        // Rekursiver Abgleich aller Angreifer (vollständiges SEE)...
        self.see(board, mv) >= threshold
    }

    pub fn see(&self, board: &Board, mv: &Turn) -> i16 {
        // Implementierung eines klassischen SEE-Angreifer-Abgleichs unter Nutzung der indizierten Bitboards
        // 1. Initialisiere Angreifer-Bitboard für das Zielfeld
        // 2. Schleife: Finde kleinsten Angreifer für die aktive Farbe, ziehe ihn ab, wechsle Farbe
        // 3. Minimax-Auswertung der Abtauschkette rückwärts
        0 // Dummy Rückgabe
    }
}
```

**Integration in Quiescence Search:**
```rust
// In search_service.rs -> quiescence-Schleife
for i in 0..turns.len {
    let capture_turn = &turns.moves[i];
    
    // Delta Pruning
    let gain = self.get_piece_value(board.get_piece_at(capture_turn.to));
    if !in_check && stand_pat + gain + 200 < alpha {
        continue; // Delta Pruning Cutoff
    }
    
    // SEE Filterung
    if !in_check && !self.see_ge(board, capture_turn, 0) {
        continue; // Schlechter Schlagzug, überspringen
    }
    
    // ... Zug ausführen und rekursiv suchen
}
```

---

## Optimierung 2: Dynamische LMR mit logarithmischer Tabelle & History-Koppelung

### 1. Konzeptionelle Grundlagen
Late Move Reductions (LMR) reduzieren die Suchtiefe für leise (quiet) Züge, die sich weit hinten in der sortierten Zugliste befinden, da diese statistisch gesehen sehr selten die besten Züge sind.
*   **Problem:** SupraH verwendet momentan eine statische Reduktion um 1 Ply ab dem 4. Zug (`turn_counter > 3`). In tiefen Suchen (z. B. Tiefe 10+) ist eine statische Reduktion um 1 Ply zu ineffizient; zudem werden gute Züge (z. B. Killer-Moves) fälschlicherweise genauso reduziert wie schlechte Züge.
*   **Lösung:** 
    1.  Vorberechnung einer **logarithmischen LMR-Tabelle** (Stockfish-Style), bei der die Reduktion stetig mit der Tiefe und dem Zug-Index wächst.
    2.  **Dynamische Modifikation:** Verringere die Reduktion für historisch starke Züge oder Killer-Moves. Erhöhe die Reduktion für historisch extrem schwache Züge.

### 2. Mathematische Grundlagen
Die LMR-Tabelle wird beim Starten der Engine initialisiert:

$$\text{LMR}[d][m] = \text{round}\left( \frac{\ln(d) \times \ln(m)}{c} \right)$$

*   $d$: Suchtiefe (1 bis 64)
*   $m$: Zug-Index in der Schleife (1 bis 64)
*   $c$: Tuning-Konstante (meist zwischen $1.8$ und $2.3$)

### 3. Rust-Entwurf (Pseudocode)

```rust
// In src/config.rs oder als globaler lazy_static in src/search_service.rs
pub struct LmrTable {
    table: [[i16; 64]; 64],
}

impl LmrTable {
    pub fn new() -> Self {
        let mut table = [[0i16; 64]; 64];
        for depth in 1..64 {
            for move_idx in 1..64 {
                let d = depth as f64;
                let m = move_idx as f64;
                // Logarithmische Verteilung
                let reduction = (d.ln() * m.ln() / 1.95) as i16;
                table[depth][move_idx] = reduction.max(0);
            }
        }
        LmrTable { table }
    }
}
```

**Koppelung mit der History-Tabelle im Negamax/Minimax:**
```rust
// In search_service.rs -> minimax Loop

let mut reduction = 0;
if config.enable_lmr && depth >= 3 && turn_counter > 3 && !is_tactical && !in_check {
    let d_idx = (depth as usize).min(63);
    let m_idx = (turn_counter as usize).min(63);
    reduction = LMR_TABLE.table[d_idx][m_idx];

    // PV-Knoten vorsichtiger reduzieren
    if is_pv {
        reduction = reduction.saturating_sub(1);
    }

    // Killer-Moves weniger stark reduzieren
    if Some(*current_turn) == killer_moves[ply as usize][0] || Some(*current_turn) == killer_moves[ply as usize][1] {
        reduction = reduction.saturating_sub(1);
    }

    // History-Koppelung: Verringere Reduktion für gute Züge, erhöhe für schlechte
    let hist_val = history_table[current_turn.from as usize][current_turn.to as usize];
    if hist_val > 4000 {
        reduction = reduction.saturating_sub(1);
    } else if hist_val < 500 {
        reduction = reduction.saturating_add(1);
    }

    // Begrenzung: Mindestens 1 Ply reduzieren (sonst kein LMR), aber nicht unter Tiefe 1 reduzieren
    reduction = reduction.clamp(1, depth - 2);
}
```

---

## Optimierung 3: Counter-Moves Heuristic & History-Malus in der Zugordnung

### 1. Konzeptionelle Grundlagen
Die Zugordnung entscheidet über die Geschwindigkeit von Alpha-Beta-Suchen. Finden wir den besten Zug zuerst, bricht die Suche sofort ab (Cutoff).
*   **Problem:** SupraH nutzt bisher nur Killer-Moves und einen positiven History-Score. Leise Züge, die wiederholt zu Fehlschlägen führten, werden nicht bestraft und drängeln sich in der Zugliste nach vorne. Außerdem fehlt ein direkter "Antwort-Zusammenhang".
*   **Lösung:**
    1.  **Counter-Moves (Antwort-Züge):** Wir speichern in einer zweidimensionalen Tabelle `counter_moves[64][64] = Option<Turn>`. Wenn der gegnerische Zug von Feld A nach Feld B ging, speichern wir den Zug C $\rightarrow$ D ab, der in dieser Situation am häufigsten zu einem Beta-Cutoff führte.
    2.  **History-Malus:** Wenn ein stiller Zug gesucht wird und *keinen* Cutoff erzeugt, ziehen wir Punkte von seinem History-Score ab.

### 2. Rust-Entwurf (Pseudocode)

```rust
// Im EngineState oder SearchContext definieren
pub struct SearchContext<'a> {
    // ...
    pub counter_moves: &'a [[Option<Turn>; 64]; 64],
}

// In search_service.rs -> minimax-Schleife (bei Beta-Cutoff für stillen Zug)
if beta_cutoff && current_turn.capture == 0 {
    // 1. Killer-Move Update (bereits vorhanden)
    // ...

    // 2. Counter-Move Speicherung
    if ply > 0 {
        let prev_turn = info.prev_moves[(ply - 1) as usize];
        if let Some(prev) = prev_turn {
            let p_from = prev.from as usize;
            let p_to = prev.to as usize;
            counter_moves[p_from][p_to] = Some(*current_turn);
        }
    }

    // 3. History-Bonus (bereits vorhanden)
    // history_table[from][to] += depth * depth;

    // 4. History-Malus für alle bisherigen Züge in diesem Ply, die NICHT den Cutoff erzeugt haben
    for j in 0..searched_moves_in_ply.len() {
        let bad_move = searched_moves_in_ply[j];
        if bad_move.capture == 0 {
            let b_from = bad_move.from as usize;
            let b_to = bad_move.to as usize;
            let penalty = (depth * depth) as u32;
            history_table[b_from][b_to] = history_table[b_from][b_to].saturating_sub(penalty);
        }
    }
}
```

**Nutzung bei der Zug-Sortierung:**
```rust
// Bei der Zuweisung von Rängen in move_gen_service.rs / get_moves:
let mut rank = 0;

if Some(m) == tt_move {
    rank += 10_000_000;
}
if Some(m) == killer_moves[ply][0] {
    rank += 900_000;
} else if Some(m) == killer_moves[ply][1] {
    rank += 800_000;
}

// Counter-Move Bonus
if ply > 0 {
    if let Some(prev) = prev_move {
        if Some(m) == counter_moves[prev.from as usize][prev.to as usize] {
            rank += 750_000;
        }
    }
}

// Addiere History Heuristik (kann durch Malus negativ/sehr gering sein)
rank += history_table[m.from as usize][m.to as usize] as i32;
```

---

## Optimierung 4: Dynamisches NMP mit Verifikationssuche

### 1. Konzeptionelle Grundlagen
Null Move Pruning (NMP) erlaubt es, die Suche drastisch abzukürzen, wenn der "geschenkte" Zug des Gegners selbst bei voller Tiefe keine Gefahr darstellt.
*   **Problem:** SupraH verwendet eine statische Reduktion um 2 Ply. Bei großen Tiefen (z. B. Tiefe 15) ist eine Reduktion um 2 Ply viel zu schwach, wodurch wertvolle NPS verschenkt werden. Gleichzeitig droht im späten Endspiel oder in Zugzwang-Situationen der "Null-Move-Blunder", wenn die Engine fälschlicherweise glaubt, die Stellung sei absolut sicher.
*   **Lösung:**
    1.  **Dynamische NMP-Reduktion:** Die Reduktion $R$ wächst adaptiv mit der Resttiefe:
        
        $$R = 2 + \frac{\text{depth}}{3}$$
        
        Bei Tiefe 3 reduzieren wir um 3, bei Tiefe 9 um 5.
    2.  **Verifikationssuche:** Wenn der Null-Move-Abbruch fehlschlägt ($\ge \beta$), führen wir zur Absicherung eine schnelle Verifikationssuche mit stark reduzierter Tiefe durch. Nur wenn auch diese $\ge \beta$ liefert, gilt der Knoten als abgeschnitten.

### 2. Rust-Entwurf (Pseudocode)

```rust
// In search_service.rs -> minimax NMP-Sektion
if config.enable_nmp && depth >= 3 && !turn.gives_check && self.has_non_pawn_material(board, board.white_to_move) {
    // 1. Berechne dynamische Reduktion
    let reduction = 2 + (depth / 3);
    let reduced_depth = depth - 1 - reduction;

    // 2. Führe Null-Move aus
    let old_white = board.white_to_move;
    board.white_to_move = !board.white_to_move;
    board.cached_hash = crate::zobrist::gen(board);

    let mut null_pv = [None; 128];
    let null_eval = if white {
        self.minimax(board, turn, reduced_depth, false, beta - 1, beta, ...).1
    } else {
        self.minimax(board, turn, reduced_depth, true, alpha, alpha + 1, ...).1
    };

    // Undo Null-Move
    board.white_to_move = old_white;
    board.cached_hash = crate::zobrist::gen(board);

    // 3. Auswertung & Verifikationssuche
    if white && null_eval >= beta {
        // Führe Verifikationssuche durch zur Absicherung (Verhindert Zugzwang-Fehler)
        let verify_depth = depth - 1 - reduction;
        let verify_eval = self.minimax(board, turn, verify_depth, true, beta - 1, beta, ...).1;
        if verify_eval >= beta {
            return (None, beta); // Sicherer Cutoff!
        }
    } else if !white && null_eval <= alpha {
        let verify_depth = depth - 1 - reduction;
        let verify_eval = self.minimax(board, turn, verify_depth, false, alpha, alpha + 1, ...).1;
        if verify_eval <= alpha {
            return (None, alpha); // Sicherer Cutoff!
        }
    }
}
```

---

## Zusammenfassung & Erwartete Auswirkung

| Optimierung | Erwarteter ELO-Gewinn | Primäre Auswirkung | Risiko / Komplexität |
| :--- | :--- | :--- | :--- |
| **1. SEE & Delta Pruning** | **+50 bis +80 ELO** | NPS-Gewinn von 30 % durch sofortiges Verwerfen schlechter Schlagzüge in der Ruhesuche. | Gering (taktisch sehr sicher). |
| **2. Dynamische LMR** | **+60 bis +100 ELO** | Exponentielles Durchdringen tiefer Suchbäume. Sucht 3-5 Ply tiefer bei gleicher Zeit. | Mittel (Margen müssen per SPSA feinabgestimmt werden). |
| **3. Counter-Moves & Malus** | **+30 bis +50 ELO** | Erhöhung der First-Move-Cutoff-Wahrscheinlichkeit. Reduziert Gesamtsuchbaum. | Gering (rein informelle Heuristik). |
| **4. Dynamisches NMP + Verify** | **+40 bis +70 ELO** | Aggressivere, aber gleichzeitig weitaus sicherere Suchen in strategischen Stellungen. | Hoch (Verifikation verhindert Zugzwang-Bugs im Endspiel). |

Durch die Kombination dieser vier Säulen kann SupraHs taktisches Fundament auf das Niveau moderner Spitzen-Engines gehoben werden, was der HCE-Engine dabei hilft, die taktischen Schwachstellen klassischer Suchbäume endgültig auszugleichen.
