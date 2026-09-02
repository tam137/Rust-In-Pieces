use rand::Rng;
use once_cell::sync::Lazy;

use crate::zobrist;
use crate::config::Config;
use crate::model::{
    Board, GameStatus, Stats, Turn, SearchContext,
    WHITE_PAWN, WHITE_ROOK, WHITE_KNIGHT, WHITE_BISHOP, WHITE_QUEEN, WHITE_KING,
    BLACK_PAWN, BLACK_ROOK, BLACK_KNIGHT, BLACK_BISHOP, BLACK_QUEEN, BLACK_KING,
};


static KNIGHT_ATTACKS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut attacks = [0u64; 64];
    let offsets = [
        (-2, -1), (-2, 1), (-1, -2), (-1, 2),
        (1, -2), (1, 2), (2, -1), (2, 1)
    ];
    for (sq, attack_mask) in attacks.iter_mut().enumerate() {
        let file = (sq % 8) as i32;
        let rank = (sq / 8) as i32;
        let mut mask = 0u64;
        for &(df, dr) in &offsets {
            let f = file + df;
            let r = rank + dr;
            if (0..8).contains(&f) && (0..8).contains(&r) {
                mask |= 1u64 << (r * 8 + f);
            }
        }
        *attack_mask = mask;
    }
    attacks
});

static KING_ATTACKS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut attacks = [0u64; 64];
    for (sq, attack_mask) in attacks.iter_mut().enumerate() {
        let file = (sq % 8) as i32;
        let rank = (sq / 8) as i32;
        let mut mask = 0u64;
        for df in -1..=1 {
            for dr in -1..=1 {
                if df == 0 && dr == 0 { continue; }
                let f = file + df;
                let r = rank + dr;
                if (0..8).contains(&f) && (0..8).contains(&r) {
                    mask |= 1u64 << (r * 8 + f);
                }
            }
        }
        *attack_mask = mask;
    }
    attacks
});

/// Ray geometry between every pair of squares, used to decide legality and discovered checks
/// without playing a move.
struct RayTables {
    /// The complete line through both squares when they share a rook or bishop ray, otherwise 0.
    /// A pinned piece may move anywhere on `line[king][piece]` and nowhere else.
    line: [[u64; 64]; 64],
    /// The squares strictly between two squares on a shared ray, otherwise 0. Used to build the
    /// check mask and to find the single blocker that makes a pin or a discovery candidate.
    between: [[u64; 64]; 64],
}

static RAYS: Lazy<RayTables> = Lazy::new(|| {
    const DIRECTIONS: [(i32, i32); 8] = [
        (1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1),
    ];
    let mut line = [[0u64; 64]; 64];
    let mut between = [[0u64; 64]; 64];

    for from in 0..64usize {
        let from_file = (from % 8) as i32;
        let from_rank = (from / 8) as i32;

        for &(df, dr) in &DIRECTIONS {
            // Walk outwards from `from` and remember the squares in order, so that the squares
            // already passed are exactly the ones between `from` and the current square.
            let mut forward = [0usize; 7];
            let mut forward_len = 0;
            let mut ray = 0u64;
            let (mut file, mut rank) = (from_file + df, from_rank + dr);
            while (0..8).contains(&file) && (0..8).contains(&rank) {
                let square = (rank * 8 + file) as usize;
                forward[forward_len] = square;
                forward_len += 1;
                ray |= 1u64 << square;
                file += df;
                rank += dr;
            }

            // The opposite direction completes the line through `from`.
            let (mut file, mut rank) = (from_file - df, from_rank - dr);
            while (0..8).contains(&file) && (0..8).contains(&rank) {
                ray |= 1u64 << ((rank * 8 + file) as usize);
                file -= df;
                rank -= dr;
            }

            let full_line = ray | (1u64 << from);
            let mut passed = 0u64;
            for &to in &forward[0..forward_len] {
                line[from][to] = full_line;
                between[from][to] = passed;
                passed |= 1u64 << to;
            }
        }
    }

    RayTables { line, between }
});

/// The complete line through `from` and `to`, or 0 when they do not share a ray.
#[inline(always)]
fn line_bb(from: u8, to: u8) -> u64 {
    RAYS.line[from as usize][to as usize]
}

/// The squares strictly between `from` and `to`, or 0 when they do not share a ray.
#[inline(always)]
fn between_bb(from: u8, to: u8) -> u64 {
    RAYS.between[from as usize][to as usize]
}

/// The squares from which a pawn of the given colour attacks `target`.
#[inline(always)]
fn pawn_check_squares(target: u8, white: bool) -> u64 {
    let file = target % 8;
    let mut squares = 0u64;
    if white {
        if file > 0 && target >= 9 {
            squares |= 1u64 << (target - 9);
        }
        if file < 7 && target >= 7 {
            squares |= 1u64 << (target - 7);
        }
    } else {
        if file > 0 && target <= 56 {
            squares |= 1u64 << (target + 7);
        }
        if file < 7 && target <= 54 {
            squares |= 1u64 << (target + 9);
        }
    }
    squares
}

pub struct NodeMasks {
    /// Square of the king of the side to move, or 64 when that king is absent from the board.
    king_sq: u8,
    /// Square of the king of the side not to move, or 64 when it is absent.
    opp_king_sq: u8,
    /// Bitboard of the enemy king, empty when it is absent. Pre-masked so that every
    /// `gives_check` test degrades to `false` instead of needing a guard.
    opp_king_bb: u64,
    /// Enemy pieces that currently attack the own king.
    checkers: u64,
    /// Squares a non-king move may land on: every square when not in check, the checker plus the
    /// squares between it and the king under a single check, and nothing at all under double
    /// check.
    check_mask: u64,
    /// Own pieces that are absolutely pinned against the own king. Such a piece may only move
    /// along `line_bb(king_sq, piece_sq)`.
    pinned: u64,
    /// Squares from which a piece of each kind — indexed by `piece % 10`, so 0 pawn through
    /// 5 king — would check the enemy king at the current occupancy. The king entry is always 0.
    check_squares: [u64; 6],
    /// Own pieces whose departure uncovers an own slider's attack on the enemy king.
    discovery: u64,
}

pub struct MoveGenService {}

impl MoveGenService {
    pub fn new() -> Self {
        // Force the ray tables to be built on the calling thread rather than inside the search,
        // where the first access would otherwise pay a 64 KB initialisation.
        Lazy::force(&RAYS);
        MoveGenService {}
    }

    pub fn get_knight_attacks(&self, sq: usize) -> u64 {
        KNIGHT_ATTACKS[sq]
    }

    pub fn get_king_attacks(&self, sq: usize) -> u64 {
        KING_ATTACKS[sq]
    }

    pub fn get_bishop_attacks(&self, square: usize, occupied: u64) -> u64 {
        crate::magic::get_bishop_attacks(square, occupied)
    }

    pub fn get_rook_attacks(&self, square: usize, occupied: u64) -> u64 {
        crate::magic::get_rook_attacks(square, occupied)
    }

    /// Own pieces that are the single occupant between `target_sq` and a slider that would
    /// otherwise attack it.
    ///
    /// Called twice per node with opposite arguments. Against the own king the sliders are the
    /// enemy's and the result is the set of absolutely pinned pieces; against the enemy king the
    /// sliders are our own and the result is the set of pieces whose departure uncovers a
    /// discovered check.
    fn blockers_toward(
        &self,
        target_sq: u8,
        own_pieces: u64,
        occupied: u64,
        diagonal_sliders: u64,
        straight_sliders: u64,
    ) -> u64 {
        // Sliders that would reach the target on an empty board are the only possible pinners.
        let mut snipers = (self.get_bishop_attacks(target_sq as usize, 0) & diagonal_sliders)
            | (self.get_rook_attacks(target_sq as usize, 0) & straight_sliders);

        let mut blockers = 0u64;
        while snipers != 0 {
            let sniper = snipers.trailing_zeros() as u8;
            snipers &= snipers - 1;

            let occupants = between_bb(target_sq, sniper) & occupied;
            // Exactly one piece in the way, and it has to be ours — a second blocker of either
            // colour makes the ray irrelevant, and an enemy blocker is not ours to move.
            if occupants.count_ones() == 1 && (occupants & own_pieces) != 0 {
                blockers |= occupants;
            }
        }
        blockers
    }

    /// Computes the per-node masks that replace `do_move`/`undo_move` during move generation.
    pub fn compute_node_masks(&self, board: &Board) -> NodeMasks {
        let white = board.white_to_move;
        let occupied = board.occupied;

        let (own_pieces, own_king_bb, opp_king_bb) = if white {
            (board.white_pieces, board.bitboards[WHITE_KING], board.bitboards[BLACK_KING])
        } else {
            (board.black_pieces, board.bitboards[BLACK_KING], board.bitboards[WHITE_KING])
        };
        // `trailing_zeros` yields 64 for an empty bitboard, which is the absent-king sentinel.
        let king_sq = own_king_bb.trailing_zeros() as u8;
        let opp_king_sq = opp_king_bb.trailing_zeros() as u8;

        let (own_bishop, own_rook, own_queen, opp_bishop, opp_rook, opp_queen) = if white {
            (WHITE_BISHOP, WHITE_ROOK, WHITE_QUEEN, BLACK_BISHOP, BLACK_ROOK, BLACK_QUEEN)
        } else {
            (BLACK_BISHOP, BLACK_ROOK, BLACK_QUEEN, WHITE_BISHOP, WHITE_ROOK, WHITE_QUEEN)
        };

        let mut checkers = 0u64;
        let mut check_mask = !0u64;
        let mut pinned = 0u64;
        if king_sq < 64 {
            checkers = self.get_attackers_mask(board, white, king_sq, occupied);
            check_mask = match checkers.count_ones() {
                0 => !0u64,
                1 => {
                    // Capture the checker or interpose on the ray to it. For a knight or pawn
                    // checker `between_bb` is empty, leaving only the capture.
                    between_bb(king_sq, checkers.trailing_zeros() as u8) | checkers
                }
                // Double check: no interposition and no capture can address both, so only the
                // king may move.
                _ => 0u64,
            };
            pinned = self.blockers_toward(
                king_sq,
                own_pieces,
                occupied,
                board.bitboards[opp_bishop] | board.bitboards[opp_queen],
                board.bitboards[opp_rook] | board.bitboards[opp_queen],
            );
        }

        let mut check_squares = [0u64; 6];
        let mut discovery = 0u64;
        if opp_king_sq < 64 {
            let target = opp_king_sq as usize;
            check_squares[0] = pawn_check_squares(opp_king_sq, white);
            check_squares[1] = self.get_rook_attacks(target, occupied);
            check_squares[2] = KNIGHT_ATTACKS[target];
            check_squares[3] = self.get_bishop_attacks(target, occupied);
            check_squares[4] = check_squares[1] | check_squares[3];
            check_squares[5] = 0;

            discovery = self.blockers_toward(
                opp_king_sq,
                own_pieces,
                occupied,
                board.bitboards[own_bishop] | board.bitboards[own_queen],
                board.bitboards[own_rook] | board.bitboards[own_queen],
            );
        }

        NodeMasks {
            king_sq,
            opp_king_sq,
            opp_king_bb,
            checkers,
            check_mask,
            pinned,
            check_squares,
            discovery,
        }
    }

    /// Whether `from` -> `to` is geometrically playable by the piece standing on `from`, ignoring
    /// king safety entirely. This is the predicate a remembered move needs before it may be
    /// played, because a Transposition Table entry survives hash collisions and can decode to a
    /// move that has nothing to do with this position.
    ///
    /// **En passant and promotions are deliberately rejected.** Neither can carry the PV/TT rank
    /// bonus in `get_valid_moves_from_move_list` — en passant moves are appended after the
    /// ranking loop and never see it, and the ranking comparison is made against a `Turn` whose
    /// `promotion` field is hard-coded to `0`, which no promotion move can equal. Accepting them
    /// here would invite a caller to short-circuit a move that the ranking loop does not sort
    /// first, which would change the move order and therefore the search tree.
    pub fn is_pseudo_legal(&self, board: &Board, from: u8, to: u8) -> bool {
        if from >= 64 || to >= 64 || from == to {
            return false;
        }

        let white = board.white_to_move;
        let piece = board.get_piece_at(from);
        let owned_by_mover = if white { (10..=15).contains(&piece) } else { (20..=25).contains(&piece) };
        if !owned_by_mover {
            return false;
        }

        let own_pieces = if white { board.white_pieces } else { board.black_pieces };
        let to_bb = 1u64 << to;
        if (own_pieces & to_bb) != 0 {
            return false;
        }

        let occupied = board.occupied;

        match piece {
            10 | 20 => {
                let from_rank = (from / 8) as i32;
                let to_rank = (to / 8) as i32;
                let from_file = (from % 8) as i32;
                let to_file = (to % 8) as i32;
                let forward = if white { 1 } else { -1 };
                let start_rank = if white { 1 } else { 6 };
                let promotion_rank = if white { 7 } else { 0 };

                // Promotions are out of scope, see the note above.
                if to_rank == promotion_rank {
                    return false;
                }

                if to_file == from_file {
                    if (occupied & to_bb) != 0 {
                        return false;
                    }
                    if to_rank == from_rank + forward {
                        return true;
                    }
                    if from_rank == start_rank && to_rank == from_rank + 2 * forward {
                        let step = (from as i32 + 8 * forward) as u8;
                        return (occupied & (1u64 << step)) == 0;
                    }
                    return false;
                }

                // A capture must be one file across, one rank forward, and must actually take a
                // piece: the empty destination of an en passant capture is rejected here.
                (to_file - from_file).abs() == 1
                    && to_rank == from_rank + forward
                    && (occupied & to_bb) != 0
            }
            11 | 21 => (self.get_rook_attacks(from as usize, occupied) & to_bb) != 0,
            12 | 22 => (KNIGHT_ATTACKS[from as usize] & to_bb) != 0,
            13 | 23 => (self.get_bishop_attacks(from as usize, occupied) & to_bb) != 0,
            14 | 24 => {
                ((self.get_rook_attacks(from as usize, occupied)
                    | self.get_bishop_attacks(from as usize, occupied))
                    & to_bb)
                    != 0
            }
            15 | 25 => {
                if (KING_ATTACKS[from as usize] & to_bb) != 0 {
                    return true;
                }
                self.is_castling_shape(board, white, from, to)
            }
            _ => false,
        }
    }

    /// Whether `from` -> `to` is the king's castling move, with the rights, the empty squares and
    /// the rook all present. Mirrors the castling clause of `generate_moves_list_for_piece`; the
    /// attacked-square test is `is_valid_castling` and is applied separately.
    fn is_castling_shape(&self, board: &Board, white: bool, from: u8, to: u8) -> bool {
        let occupied = board.occupied;
        if white && from == 4 {
            if to == 6 {
                return board.white_possible_to_castle_short
                    && (occupied & ((1u64 << 5) | (1u64 << 6))) == 0
                    && (board.bitboards[WHITE_ROOK] & (1u64 << 7)) != 0;
            }
            if to == 2 {
                return board.white_possible_to_castle_long
                    && (occupied & ((1u64 << 1) | (1u64 << 2) | (1u64 << 3))) == 0
                    && (board.bitboards[WHITE_ROOK] & 1u64) != 0;
            }
        } else if !white && from == 60 {
            if to == 62 {
                return board.black_possible_to_castle_short
                    && (occupied & ((1u64 << 61) | (1u64 << 62))) == 0
                    && (board.bitboards[BLACK_ROOK] & (1u64 << 63)) != 0;
            }
            if to == 58 {
                return board.black_possible_to_castle_long
                    && (occupied & ((1u64 << 57) | (1u64 << 58) | (1u64 << 59))) == 0
                    && (board.bitboards[BLACK_ROOK] & (1u64 << 56)) != 0;
            }
        }
        false
    }

    /// Validates a remembered PV or Transposition Table move and returns it fully formed — with
    /// `capture`, `gives_check` and the exact rank the ranking loop would have given it — or
    /// `None` if it may not be searched ahead of generation.
    ///
    /// This is Stage 0 of the `MovePicker` in `task.md` 1.2.2. It is the only stage that is
    /// order-preserving: the PV/TT move is ranked at 170,000 or above while every other move is
    /// bounded above by 140,000 (a queen capture at 90,000 plus a check at 50,000), so it always
    /// sorted first anyway and searching it before generating anything leaves the tree identical.
    pub fn build_stage0_move(
        &self,
        board: &Board,
        masks: &NodeMasks,
        candidate: &Turn,
        config: &Config,
    ) -> Option<Turn> {
        // A promotion never matches the ranking comparison, which is made against a `Turn` with
        // `promotion == 0`, so it is never the bonused move. `is_pseudo_legal` rejects the
        // geometry as well; this rejects an entry that carries a promotion piece outright.
        if candidate.promotion != 0 {
            return None;
        }

        let white = board.white_to_move;
        let from = candidate.from;
        let to = candidate.to;

        if !self.is_pseudo_legal(board, from, to) {
            return None;
        }

        let king_sq = masks.king_sq;
        let is_king_move = from == king_sq;

        // Under double check nothing but a king move can be legal.
        if masks.checkers.count_ones() > 1 && !is_king_move {
            return None;
        }

        let to_bb = 1u64 << to;
        if is_king_move {
            let castling = (to as i32 - from as i32).abs() == 2;
            if castling {
                if !self.is_valid_castling(board, white, to as i32, masks) {
                    return None;
                }
            } else {
                // Kings may not stand next to each other, and the king is lifted out of the
                // occupancy for the attack test so it cannot shield its own destination square
                // from the slider that is checking it.
                if masks.opp_king_sq < 64 && (KING_ATTACKS[masks.opp_king_sq as usize] & to_bb) != 0 {
                    return None;
                }
                let occupied_without_king = board.occupied ^ (1u64 << from);
                if self.get_attackers_mask(board, white, to, occupied_without_king) != 0 {
                    return None;
                }
            }
        } else {
            // Same legality mask the generator applies: answer the check, and stay on the pin
            // line if pinned.
            let mut legal_mask = masks.check_mask;
            if (masks.pinned & (1u64 << from)) != 0 {
                legal_mask &= line_bb(king_sq, from);
            }
            if (legal_mask & to_bb) == 0 {
                return None;
            }
        }

        let capture = board.get_piece_at(to);
        let mut turn = Turn::new(from, to, capture, 0, false, 0);
        turn.rank = crate::model::BAND_TT;
        turn.gives_check = self.gives_check(board, &turn, masks);
        if turn.gives_check {
            turn.rank += config.give_check_rank_bonus * 10000;
        }
        Some(turn)
    }

    /// Generates a list of valid capture moves for a given board state.
    pub fn generate_valid_moves_list_capture(
        &self,
        board: &mut Board,
        stats: &mut Stats,
        config: &Config,
        context: &SearchContext,
        do_move_ordering: bool,
        valid_moves: &mut crate::model::MoveList,
    ) {
        if board.game_status != GameStatus::Normal {
            return;
        }
        let masks = self.compute_node_masks(board);
        let mut move_list = crate::model::MoveRawList::new();
        self.generate_moves_list_for_piece(board, 0, true, &masks, &mut move_list);
        let start_len = valid_moves.len;
        self.get_valid_moves_from_move_list(&move_list, board, stats, config, true, context, do_move_ordering, &masks, false, false, valid_moves);

        stats.add_created_capture_nodes(valid_moves.len - start_len);
    }

    /// Generates a list of valid moves for a given board state.
    pub fn generate_valid_moves_list(
        &self,
        board: &mut Board,
        stats: &mut Stats,
        config: &Config,
        context: &SearchContext,
        do_move_ordering: bool,
        valid_moves: &mut crate::model::MoveList,
    ) {
        if board.game_status != GameStatus::Normal {
            return;
        }
        let masks = self.compute_node_masks(board);
        let mut move_list = crate::model::MoveRawList::new();
        self.generate_moves_list_for_piece(board, 0, false, &masks, &mut move_list);
        self.get_valid_moves_from_move_list(
            &move_list, board, stats, config, false, context, do_move_ordering, &masks, false,
            false, valid_moves);
    }

    /// Stage 1 of the `MovePicker`: every capture, en passant included, appended to the list.
    ///
    /// `generate_valid_moves_list_capture` cannot serve here. It is the Quiescence Search's
    /// generator and leaves en passant out, and en passant is a capture: it ranks in the capture
    /// band and has to be searched with the rest of them, not with the quiet moves two stages
    /// later. No move is given the table-move band, which is stage 0's business.
    pub fn append_capture_stage(
        &self,
        board: &mut Board,
        stats: &mut Stats,
        config: &Config,
        context: &SearchContext,
        do_move_ordering: bool,
        valid_moves: &mut crate::model::MoveList,
    ) {
        if board.game_status != GameStatus::Normal {
            return;
        }
        let masks = self.compute_node_masks(board);
        let mut move_list = crate::model::MoveRawList::new();
        // The raw pass has to see every move, not only the captures: a pawn push to the last rank
        // takes nothing and still ranks a band above them. What the capture stage saves is the
        // per-move work behind it -- legality, the check test and the ranking -- which the filter
        // below skips for every quiet move that is not a promotion.
        self.generate_moves_list_for_piece(board, 0, false, &masks, &mut move_list);
        self.get_valid_moves_from_move_list(
            &move_list, board, stats, config, true, context, do_move_ordering, &masks, true, true,
            valid_moves);
    }

    /// Validates a remembered move -- the table move for stage 0, a killer or the counter move
    /// for stage 2 -- and returns it fully formed at the rank the ranking loop would have given
    /// it, or `None` if it may not be searched ahead of generation.
    pub fn build_remembered_move(
        &self,
        board: &Board,
        masks: &NodeMasks,
        candidate: &Turn,
        config: &Config,
        rank: i32,
    ) -> Option<Turn> {
        let mut turn = self.build_stage0_move(board, masks, candidate, config)?;
        turn.rank = rank;
        if turn.gives_check {
            turn.rank += config.give_check_rank_bonus * 10000;
        }
        Some(turn)
    }

    /// Generation for the refill of a staged node: the same list, ranked the same way, except
    /// that no move is given the table-move band.
    ///
    /// The table move has already been searched by then, and the table itself has moved on --
    /// the searched move's own subtree writes to it, and can evict this node's entry. Probing it
    /// again would hand the band to whatever move the table holds now, which is not a move the
    /// eager path would ever have put there.
    pub fn generate_valid_moves_list_without_table_move(
        &self,
        board: &mut Board,
        stats: &mut Stats,
        config: &Config,
        context: &SearchContext,
        do_move_ordering: bool,
        valid_moves: &mut crate::model::MoveList,
    ) {
        if board.game_status != GameStatus::Normal {
            return;
        }
        let masks = self.compute_node_masks(board);
        let mut move_list = crate::model::MoveRawList::new();
        self.generate_moves_list_for_piece(board, 0, false, &masks, &mut move_list);
        self.get_valid_moves_from_move_list(
            &move_list, board, stats, config, false, context, do_move_ordering, &masks, true,
            false, valid_moves);
    }

    fn get_valid_moves_from_move_list(
        &self,
        move_list: &crate::model::MoveRawList,
        board: &mut Board,
        stats: &mut Stats,
        config: &Config,
        only_captures: bool,
        context: &SearchContext,
        do_move_ordering: bool,
        masks: &NodeMasks,
        skip_table_move: bool,
        capture_stage: bool,
        valid_moves: &mut crate::model::MoveList,
    ) {
        let white_turn = board.white_to_move;
        let king_value = if white_turn { 15 } else { 25 };

        // get pv node
        let mut pv_node = None;
        if !only_captures && !skip_table_move && config.use_pv_nodes {
            if board.cached_hash == 0 {
                board.cached_hash = zobrist::gen_hash(board);
            }
            let pv_nodes_guard = context.pv_nodes.lock().expect(crate::model::RIP_COULDN_LOCK_MUTEX);
            if let Some(pv_node_result) = pv_nodes_guard.get(&board.cached_hash) {
                pv_node = Some(*pv_node_result);
            }
        }

        let mut tt_best_move = None;
        if !only_captures && !skip_table_move && config.use_zobrist {
            if board.cached_hash == 0 {
                board.cached_hash = zobrist::gen_hash(board);
            }
            if let Some(entry) = context.zobrist_table.get_entry(&board.cached_hash) {
                tt_best_move = entry.decompress_move(board);
            }
        }

        for i in (0..move_list.len).step_by(2) {
            let idx0 = move_list.moves[i];
            let idx1 = move_list.moves[i + 1];

            let capture = board.get_piece_at(idx1);
            if capture == 0 && only_captures {
                // A promotion ranks a band above the captures, so it belongs to the capture stage
                // even when it takes nothing. Leaving it to the quiet stage would search it after
                // moves it outranks.
                let promotes = capture_stage
                    && self.get_promotion_move(board, white_turn, idx0 as i32, idx1 as i32).is_some();
                if !promotes {
                    continue;
                }
            }

            let mut move_turn = Turn::new(idx0, idx1, capture, 0, false, 0);

            let is_table_move = if let Some(pv) = &pv_node {
                *pv == move_turn
            } else if let Some(tt_move) = &tt_best_move {
                *tt_move == move_turn
            } else {
                false
            };

            // Check for castling
            let moved_piece = board.get_piece_at(idx0);
            if !only_captures && (moved_piece == king_value && (idx1 as i32 - idx0 as i32).abs() == 2) && !self.is_valid_castling(board, white_turn, idx1 as i32, masks) {
                continue;
            }

            // Most Valuable Victim less Least Valuable Attacker, the score a capture carries
            // inside its band. A promotion carries it too, one band higher.
            let capture_score = if move_turn.capture == 0 {
                0
            } else {
                let victim = match move_turn.capture {
                    10 | 20 => 20000,
                    11 | 21 => 50000,
                    12 | 22 => 30000,
                    13 | 23 => 30000,
                    14 | 24 => 90000,
                    _ => 0,
                };
                let attacker = match board.get_piece_at(move_turn.from) {
                    11 | 21 => -10000,
                    14 | 24 => -30000,
                    _ => 0,
                };
                victim + attacker
            };

            // The band decides the order before any score inside it does. A killer or counter
            // move is a quiet the search remembers, so it is ranked as one, above the rest of its
            // band and below every capture -- which is what lets those two classes be resolved
            // without generating a quiet move at all.
            move_turn.rank = if is_table_move {
                crate::model::BAND_TT
            } else if move_turn.capture != 0 {
                crate::model::BAND_CAPTURE + capture_score
            } else {
                let mut remembered = 0;
                if Some(move_turn) == context.killer_moves[0] {
                    remembered = config.killer_move_1_rank_bonus;
                } else if Some(move_turn) == context.killer_moves[1] {
                    remembered = config.killer_move_2_rank_bonus;
                }
                if Some(move_turn) == context.counter_move {
                    remembered = remembered.max(config.counter_move_rank_bonus);
                }

                if remembered > 0 {
                    crate::model::BAND_KILLER + remembered
                } else {
                    let from = move_turn.from as usize;
                    let to = move_turn.to as usize;
                    crate::model::BAND_QUIET + unsafe { (*context.history_table)[from][to] } as i32
                }
            };

            // Check for promotion
            if let Some(promotion_move) = self.get_promotion_move(board, white_turn, idx0 as i32, idx1 as i32) {
                move_turn.promotion = promotion_move.promotion;
                self.add_promotion_moves(
                    board,
                    &mut move_turn,
                    config,
                    valid_moves,
                    white_turn,
                    masks,
                    capture_score,
                );
            } else {
                self.add_move(board, &mut move_turn, config, valid_moves, masks);
            }
        }

        // Add en passant moves. They are the one move type that cannot be settled by the pin and
        // check masks, because two squares vacate at once, so each candidate gets an exact test
        // against the occupancy it would produce.
        if !only_captures || capture_stage {
            let en_passante_turns = self.get_en_passante_turns(board, white_turn);
            for opt_turn in &en_passante_turns {
                if let Some(mut turn) = *opt_turn {
                    if self.is_en_passant_legal(board, &turn, white_turn, masks) {
                        self.add_move(board, &mut turn, config, valid_moves, masks);
                    }
                }
            }
        }

        // Move sorting
        if config.pre_sort_moves {
            let slice = &mut valid_moves.moves[0..valid_moves.len];
            if do_move_ordering {
                slice.sort_unstable_by(|a, b| b.rank.cmp(&a.rank));
            } else {
                let mut rng = rand::thread_rng();
                let mut noisy_ranks = [0i32; 256];
                for idx in 0..valid_moves.len {
                    let noise = rng.gen_range(-config.smp_thread_eval_noise..=config.smp_thread_eval_noise) as i32;
                    noisy_ranks[idx] = slice[idx].rank + noise;
                }
                let mut indices: [usize; 256] = [0; 256];
                for (idx, item) in indices.iter_mut().enumerate().take(valid_moves.len) {
                    *item = idx;
                }
                let active_indices = &mut indices[0..valid_moves.len];
                active_indices.sort_unstable_by(|&a, &b| noisy_ranks[b].cmp(&noisy_ranks[a]));
                
                let mut temp_moves = [Turn::new(0, 0, 0, 0, false, 0); 256];
                for idx in 0..valid_moves.len {
                    temp_moves[idx] = slice[active_indices[idx]];
                }
                slice.copy_from_slice(&temp_moves[0..valid_moves.len]);
            }
        }

        // Check GameStatus
        if valid_moves.is_empty() && !only_captures {
            if masks.checkers != 0 {
                board.game_status = if board.white_to_move {
                    GameStatus::BlackWin
                } else {
                    GameStatus::WhiteWin
                };
            } else {
                board.game_status = GameStatus::Draw;
            }
        }

        stats.add_created_nodes(valid_moves.len);
        if valid_moves.len > config.truncate_bad_moves {
            valid_moves.len = config.truncate_bad_moves;
        }
    }

    #[allow(unused_assignments)]
    fn get_en_passante_turns(&self, board: &Board, white_turn: bool) -> [Option<Turn>; 4] {
        let mut en_passante_turns = [None; 4];
        let mut idx = 0;
        if board.field_for_en_passante != -1 {
            let target_piece = if white_turn { 20 } else { 10 };
            let ep_sq = board.field_for_en_passante;
            let file = ep_sq % 8;
            if white_turn {
                if file > 0 {
                    let from = ep_sq - 9;
                    if (board.bitboards[WHITE_PAWN] & (1u64 << from)) != 0 {
                        en_passante_turns[idx] = Some(Turn::new(from as u8, ep_sq as u8, target_piece, 0, false, 0));
                        idx += 1;
                    }
                }
                if file < 7 {
                    let from = ep_sq - 7;
                    if (board.bitboards[WHITE_PAWN] & (1u64 << from)) != 0 {
                        en_passante_turns[idx] = Some(Turn::new(from as u8, ep_sq as u8, target_piece, 0, false, 0));
                        idx += 1;
                    }
                }
            } else {
                if file > 0 {
                    let from = ep_sq + 7;
                    if (board.bitboards[BLACK_PAWN] & (1u64 << from)) != 0 {
                        en_passante_turns[idx] = Some(Turn::new(from as u8, ep_sq as u8, target_piece, 0, false, 0));
                        idx += 1;
                    }
                }
                if file < 7 {
                    let from = ep_sq + 9;
                    if (board.bitboards[BLACK_PAWN] & (1u64 << from)) != 0 {
                        en_passante_turns[idx] = Some(Turn::new(from as u8, ep_sq as u8, target_piece, 0, false, 0));
                        idx += 1;
                    }
                }
            }
        }
        en_passante_turns
    }

    /// Accepts an already legal move, flags whether it gives check, and appends it.
    ///
    /// Legality is settled during generation by the pin and check masks, so nothing here touches
    /// the board.
    fn add_move(
        &self,
        board: &Board,
        turn: &mut Turn,
        config: &Config,
        valid_moves: &mut crate::model::MoveList,
        masks: &NodeMasks,
    ) {
        turn.gives_check = self.gives_check(board, turn, masks);
        if turn.gives_check {
            turn.rank += config.give_check_rank_bonus * 10000;
        }
        valid_moves.push(*turn);
    }

    /// Decides whether a move checks the enemy king without playing it.
    ///
    /// A check is either direct — the piece arriving on `turn.to` attacks the enemy king — or
    /// discovered, when the departure of `turn.from` uncovers one of our own sliders. Castling
    /// and en passant additionally move or remove a second piece and are handled explicitly.
    fn gives_check(&self, board: &Board, turn: &Turn, masks: &NodeMasks) -> bool {
        let opp_king_bb = masks.opp_king_bb;
        if opp_king_bb == 0 {
            return false;
        }

        let from = turn.from;
        let to = turn.to;
        let from_mask = 1u64 << from;
        let to_mask = 1u64 << to;
        let moved_piece = board.get_piece_at(from);
        let is_pawn = moved_piece == 10 || moved_piece == 20;
        let enemy_king = masks.opp_king_sq as usize;

        let (own_bishop, own_rook, own_queen) = if moved_piece < 20 {
            (WHITE_BISHOP, WHITE_ROOK, WHITE_QUEEN)
        } else {
            (BLACK_BISHOP, BLACK_ROOK, BLACK_QUEEN)
        };

        // En passant vacates both the origin and the captured pawn's square, so neither the
        // precomputed check squares nor the single-square discovery test can describe it. The
        // resulting occupancy is cheap enough to evaluate outright, and the case is rare.
        if is_pawn && turn.promotion == 0 && to as i8 == board.field_for_en_passante {
            let captured_sq = if moved_piece == 10 { to - 8 } else { to + 8 };
            let occupied_after = (board.occupied ^ from_mask ^ (1u64 << captured_sq)) | to_mask;
            let diagonal = board.bitboards[own_bishop] | board.bitboards[own_queen];
            let straight = board.bitboards[own_rook] | board.bitboards[own_queen];

            return (masks.check_squares[0] & to_mask) != 0
                || (self.get_bishop_attacks(enemy_king, occupied_after) & diagonal) != 0
                || (self.get_rook_attacks(enemy_king, occupied_after) & straight) != 0;
        }

        // Direct check. For a promotion the arriving piece is not the piece that left, and the
        // vacated origin square can itself lie on the new piece's ray to the enemy king — a pawn
        // promoting on e8 with the enemy king on e1 blocks its own queen in a table built at the
        // pre-move occupancy — so promotions are evaluated against the post-departure occupancy.
        let mut checks = if turn.promotion != 0 {
            let occupied_after = board.occupied ^ from_mask;
            let target = to as usize;
            match turn.promotion % 10 {
                1 => (self.get_rook_attacks(target, occupied_after) & opp_king_bb) != 0,
                2 => (KNIGHT_ATTACKS[target] & opp_king_bb) != 0,
                3 => (self.get_bishop_attacks(target, occupied_after) & opp_king_bb) != 0,
                4 => {
                    ((self.get_rook_attacks(target, occupied_after)
                        | self.get_bishop_attacks(target, occupied_after))
                        & opp_king_bb)
                        != 0
                }
                _ => false,
            }
        } else {
            // Safe at the pre-move occupancy: the only piece whose departure could open the ray
            // between `to` and the enemy king is the mover itself, and for a slider that would
            // mean it already attacked the enemy king, which cannot happen with us to move.
            (masks.check_squares[(moved_piece % 10) as usize] & to_mask) != 0
        };

        // Castling also relocates a rook, which is what usually delivers the check.
        if !checks && (moved_piece == 15 || moved_piece == 25) && (to as i8 - from as i8).abs() == 2 {
            let rook_squares = match to {
                6 => Some((7u8, 5u8)),
                2 => Some((0u8, 3u8)),
                62 => Some((63u8, 61u8)),
                58 => Some((56u8, 59u8)),
                _ => None,
            };
            if let Some((rook_from, rook_to)) = rook_squares {
                let occupied_after = (board.occupied ^ from_mask ^ (1u64 << rook_from))
                    | to_mask
                    | (1u64 << rook_to);
                checks = (self.get_rook_attacks(rook_to as usize, occupied_after) & opp_king_bb) != 0;
            }
        }

        // Discovered check: the mover blocked one of our sliders and steps off its line. Staying
        // on the line keeps the ray blocked, which is why the alignment test is needed.
        if !checks
            && (masks.discovery & from_mask) != 0
            && (line_bb(masks.opp_king_sq, from) & to_mask) == 0
        {
            checks = true;
        }

        checks
    }

    /// Exact legality test for an en passant capture.
    ///
    /// The pin and check masks cannot express this move: it removes a pawn from a square other
    /// than its destination, which both exposes the own king along a rank and can be the very
    /// capture that answers a check. Both sides of that are settled here against the occupancy
    /// the move would produce.
    fn is_en_passant_legal(&self, board: &Board, turn: &Turn, white: bool, masks: &NodeMasks) -> bool {
        let king_sq = masks.king_sq;
        if king_sq >= 64 {
            return true;
        }

        let captured_sq = if white { turn.to - 8 } else { turn.to + 8 };
        let occupied_after = (board.occupied ^ (1u64 << turn.from) ^ (1u64 << captured_sq))
            | (1u64 << turn.to);

        let (opp_pawn, opp_knight, opp_bishop, opp_rook, opp_queen, opp_king) = if white {
            (BLACK_PAWN, BLACK_KNIGHT, BLACK_BISHOP, BLACK_ROOK, BLACK_QUEEN, BLACK_KING)
        } else {
            (WHITE_PAWN, WHITE_KNIGHT, WHITE_BISHOP, WHITE_ROOK, WHITE_QUEEN, WHITE_KING)
        };

        let diagonal = board.bitboards[opp_bishop] | board.bitboards[opp_queen];
        if (self.get_bishop_attacks(king_sq as usize, occupied_after) & diagonal) != 0 {
            return false;
        }
        let straight = board.bitboards[opp_rook] | board.bitboards[opp_queen];
        if (self.get_rook_attacks(king_sq as usize, occupied_after) & straight) != 0 {
            return false;
        }
        if (KNIGHT_ATTACKS[king_sq as usize] & board.bitboards[opp_knight]) != 0 {
            return false;
        }
        if (KING_ATTACKS[king_sq as usize] & board.bitboards[opp_king]) != 0 {
            return false;
        }
        // The captured pawn is gone, so it can no longer be the piece giving check.
        let remaining_pawns = board.bitboards[opp_pawn] & !(1u64 << captured_sq);
        pawn_check_squares(king_sq, !white) & remaining_pawns == 0
    }

    fn add_promotion_moves(
        &self,
        board: &Board,
        turn: &mut Turn,
        config: &Config,
        valid_moves: &mut crate::model::MoveList,
        white_turn: bool,
        masks: &NodeMasks,
        capture_score: i32,
    ) {
        // A queen or knight promotion is its own class and ranks above every capture. A rook or
        // bishop promotion carries no bonus and is left in the band the move already had, which
        // is where the previous scheme also left it.
        let base_rank = turn.rank;
        if config.use_underpromotions {
            let promotion_types = if white_turn { [11, 12, 13, 14] } else { [21, 22, 23, 24] };
            for &promotion in &promotion_types {
                turn.promotion = promotion;
                turn.gives_check = false;
                turn.rank = base_rank;
                match promotion {
                    11 | 21 => {} // Rook promotion
                    12 | 22 => turn.rank = crate::model::BAND_PROMOTION
                        + config.give_promotion_rank_bonus_knight * 10000 + capture_score,
                    13 | 23 => {} // Bishop promotion
                    14 | 24 => turn.rank = crate::model::BAND_PROMOTION
                        + config.give_promotion_rank_bonus_queen * 10000 + capture_score,
                    _ => panic!("Promotion value not expected: {}", promotion),
                }
                self.add_move(board, turn, config, valid_moves, masks);
            }
        } else {
            let promotion_types = if white_turn { [12, 14] } else { [22, 24] };
            for &promotion in &promotion_types {
                turn.promotion = promotion;
                turn.gives_check = false;
                turn.rank = base_rank;
                match promotion {
                    12 | 22 => turn.rank = crate::model::BAND_PROMOTION
                        + config.give_promotion_rank_bonus_knight * 10000 + capture_score,
                    14 | 24 => turn.rank = crate::model::BAND_PROMOTION
                        + config.give_promotion_rank_bonus_queen * 10000 + capture_score,
                    _ => panic!("Promotion value not expected: {}", promotion),
                }
                self.add_move(board, turn, config, valid_moves, masks);
            }
        }
    }

    fn is_valid_castling(&self, board: &Board, white_turn: bool, target: i32, masks: &NodeMasks) -> bool {
        let check_squares: &[u8] = if white_turn {
            if target == 6 { &[5, 6] } else { &[3, 2] }
        } else if target == 62 { &[61, 62] } else { &[59, 58] };

        if masks.checkers != 0 {
            return false;
        }

        for &square in check_squares {
            let attackers = self.get_attackers_mask(board, white_turn, square, board.occupied);
            if attackers != 0 {
                return false;
            }
        }

        if white_turn {
            if target == 6 && !board.white_possible_to_castle_short {
                return false;
            }
            if target == 2 && !board.white_possible_to_castle_long {
                return false;
            }
        } else {
            if target == 62 && !board.black_possible_to_castle_short {
                return false;
            }
            if target == 58 && !board.black_possible_to_castle_long {
                return false;
            }
        }
        true
    }

    fn get_promotion_move(&self, board: &Board, white_turn: bool, idx0: i32, idx1: i32) -> Option<Turn> {
        if white_turn && idx0 / 8 == 6 && board.get_piece_at(idx0 as u8) == 10 {
            Some(Turn {
                from: idx0 as u8,
                to: idx1 as u8,
                capture: 0,
                promotion: 14,
                order: 0,
                gives_check: false,
                eval: 0,
                has_hashed_eval: false,
                rank: 0,
            })
        } else if !white_turn && idx0 / 8 == 1 && board.get_piece_at(idx0 as u8) == 20 {
            Some(Turn {
                from: idx0 as u8,
                to: idx1 as u8,
                capture: 0,
                promotion: 24,
                order: 0,
                gives_check: false,
                eval: 0,
                has_hashed_eval: false,
                rank: 0,
            })
        } else {
            None
        }
    }

    /// Generates the legal moves of one side directly from the bitboards.
    ///
    /// The moves produced are legal by construction: `masks.check_mask` restricts every non-king
    /// move to a square that answers an existing check, `masks.pinned` restricts a pinned piece
    /// to the line it is pinned on, and king moves are tested against the enemy attack set with
    /// the king itself lifted out of the occupancy. Nothing here plays a move.
    pub fn generate_moves_list_for_piece(&self, board: &Board, idx: i32, only_captures: bool, masks: &NodeMasks, moves: &mut crate::model::MoveRawList) {
        let white = board.white_to_move;
        let king_sq = masks.king_sq;
        let double_check = masks.checkers.count_ones() > 1;

        let own_pieces = if white { board.white_pieces } else { board.black_pieces };
        let opp_pieces = if white { board.black_pieces } else { board.white_pieces };
        let occupied = board.occupied;
        let target_mask = if only_captures { opp_pieces } else { !own_pieces };

        let mut piece_mask = if idx > 0 {
            1u64 << (idx as u8)
        } else {
            own_pieces
        };

        let opp_king_sq = masks.opp_king_sq as usize;

        while piece_mask != 0 {
            let sq = piece_mask.trailing_zeros() as u8;
            piece_mask &= piece_mask - 1;

            if double_check && sq != king_sq {
                continue;
            }

            let piece = board.get_piece_at(sq);
            if piece == 0 {
                continue;
            }

            // Squares this piece may legally land on. The king is excluded: it is not subject to
            // the check mask — it can step off the ray rather than block it — and it cannot be
            // pinned, so its moves are filtered individually below.
            let legal_mask = if sq == king_sq {
                !0u64
            } else if (masks.pinned & (1u64 << sq)) != 0 {
                masks.check_mask & line_bb(king_sq, sq)
            } else {
                masks.check_mask
            };

            match piece {
                10 | 20 => {
                    let rank = sq / 8;
                    let file = sq % 8;
                    // The four pawn cases stay separate rather than becoming one target
                    // bitboard: their emission order — single push, double push, capture left,
                    // capture right — is part of the move ordering the search sees.
                    if white {
                        if !only_captures {
                            let to = sq + 8;
                            if to < 64 && (occupied & (1u64 << to)) == 0 {
                                if (legal_mask & (1u64 << to)) != 0 {
                                    moves.push(sq as i32);
                                    moves.push(to as i32);
                                }
                                if rank == 1 {
                                    let to_double = sq + 16;
                                    if (occupied & (1u64 << to_double)) == 0 && (legal_mask & (1u64 << to_double)) != 0 {
                                        moves.push(sq as i32);
                                        moves.push(to_double as i32);
                                    }
                                }
                            }
                        }
                        if file > 0 {
                            let to = sq + 7;
                            if to < 64 && (opp_pieces & (1u64 << to)) != 0 && (legal_mask & (1u64 << to)) != 0 {
                                moves.push(sq as i32);
                                moves.push(to as i32);
                            }
                        }
                        if file < 7 {
                            let to = sq + 9;
                            if to < 64 && (opp_pieces & (1u64 << to)) != 0 && (legal_mask & (1u64 << to)) != 0 {
                                moves.push(sq as i32);
                                moves.push(to as i32);
                            }
                        }
                    } else {
                        if !only_captures {
                            let to = sq - 8;
                            if (occupied & (1u64 << to)) == 0 {
                                if (legal_mask & (1u64 << to)) != 0 {
                                    moves.push(sq as i32);
                                    moves.push(to as i32);
                                }
                                if rank == 6 {
                                    let to_double = sq - 16;
                                    if (occupied & (1u64 << to_double)) == 0 && (legal_mask & (1u64 << to_double)) != 0 {
                                        moves.push(sq as i32);
                                        moves.push(to_double as i32);
                                    }
                                }
                            }
                        }
                        if file > 0 {
                            let to = sq - 9;
                            if (opp_pieces & (1u64 << to)) != 0 && (legal_mask & (1u64 << to)) != 0 {
                                moves.push(sq as i32);
                                moves.push(to as i32);
                            }
                        }
                        if file < 7 {
                            let to = sq - 7;
                            if (opp_pieces & (1u64 << to)) != 0 && (legal_mask & (1u64 << to)) != 0 {
                                moves.push(sq as i32);
                                moves.push(to as i32);
                            }
                        }
                    }
                }
                12 | 22 => {
                    let mut targets = KNIGHT_ATTACKS[sq as usize] & target_mask & legal_mask;
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        moves.push(sq as i32);
                        moves.push(to as i32);
                        targets &= targets - 1;
                    }
                }
                13 | 23 => {
                    let mut targets = self.get_bishop_attacks(sq as usize, occupied) & target_mask & legal_mask;
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        moves.push(sq as i32);
                        moves.push(to as i32);
                        targets &= targets - 1;
                    }
                }
                11 | 21 => {
                    let mut targets = self.get_rook_attacks(sq as usize, occupied) & target_mask & legal_mask;
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        moves.push(sq as i32);
                        moves.push(to as i32);
                        targets &= targets - 1;
                    }
                }
                14 | 24 => {
                    let bishop_attacks = self.get_bishop_attacks(sq as usize, occupied);
                    let rook_attacks = self.get_rook_attacks(sq as usize, occupied);
                    let mut targets = (bishop_attacks | rook_attacks) & target_mask & legal_mask;
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        moves.push(sq as i32);
                        moves.push(to as i32);
                        targets &= targets - 1;
                    }
                }
                15 | 25 => {
                    let mut targets = KING_ATTACKS[sq as usize] & target_mask;
                    if opp_king_sq < 64 {
                        targets &= !KING_ATTACKS[opp_king_sq];
                    }
                    // The king is lifted out of the occupancy before the attack test, otherwise
                    // it would shield the square it is stepping onto from the very slider that
                    // is checking it — the classic "king retreats along the ray" bug.
                    let occupied_without_king = occupied ^ (1u64 << sq);
                    while targets != 0 {
                        let to = targets.trailing_zeros() as u8;
                        if self.get_attackers_mask(board, white, to, occupied_without_king) == 0 {
                            moves.push(sq as i32);
                            moves.push(to as i32);
                        }
                        targets &= targets - 1;
                    }

                    if !only_captures {
                        if white {
                            if sq == 4 {
                                if board.white_possible_to_castle_short
                                    && (occupied & ((1u64 << 5) | (1u64 << 6))) == 0
                                    && (board.bitboards[WHITE_ROOK] & (1u64 << 7)) != 0
                                {
                                    moves.push(4);
                                    moves.push(6);
                                }
                                if board.white_possible_to_castle_long
                                    && (occupied & ((1u64 << 1) | (1u64 << 2) | (1u64 << 3))) == 0
                                    && (board.bitboards[WHITE_ROOK] & (1u64 << 0)) != 0
                                {
                                    moves.push(4);
                                    moves.push(2);
                                }
                            }
                        } else if sq == 60 {
                            if board.black_possible_to_castle_short
                                && (occupied & ((1u64 << 61) | (1u64 << 62))) == 0
                                && (board.bitboards[BLACK_ROOK] & (1u64 << 63)) != 0
                            {
                                moves.push(60);
                                moves.push(62);
                            }
                            if board.black_possible_to_castle_long
                                && (occupied & ((1u64 << 57) | (1u64 << 58) | (1u64 << 59))) == 0
                                && (board.bitboards[BLACK_ROOK] & (1u64 << 56)) != 0
                            {
                                moves.push(60);
                                moves.push(58);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn is_pinned_away_from_target(&self, board: &Board, sq: u8, target_idx: u8, white: bool) -> bool {
        let king_bb = if white {
            board.bitboards[WHITE_KING]
        } else {
            board.bitboards[BLACK_KING]
        };
        let king_sq = king_bb.trailing_zeros() as u8;
        if king_sq >= 64 {
            return false;
        }

        let opp_bishop = if white { BLACK_BISHOP } else { WHITE_BISHOP };
        let opp_rook = if white { BLACK_ROOK } else { WHITE_ROOK };
        let opp_queen = if white { BLACK_QUEEN } else { WHITE_QUEEN };

        // 1. Check diagonal pins
        let original_diag_attacks = self.get_bishop_attacks(king_sq as usize, board.occupied);
        let original_diag_attackers = original_diag_attacks & (board.bitboards[opp_bishop] | board.bitboards[opp_queen]);
        
        let occupied_without_sq = board.occupied & !(1u64 << sq);
        let diag_attacks = self.get_bishop_attacks(king_sq as usize, occupied_without_sq);
        let diag_attackers = diag_attacks & (board.bitboards[opp_bishop] | board.bitboards[opp_queen]);
        
        if original_diag_attackers == 0 && diag_attackers != 0 {
            let target_mask = 1u64 << target_idx;
            if (diag_attacks & target_mask) == 0 {
                return true; // Absolutely pinned away from target
            }
        }

        // 2. Check straight pins
        let original_straight_attacks = self.get_rook_attacks(king_sq as usize, board.occupied);
        let original_straight_attackers = original_straight_attacks & (board.bitboards[opp_rook] | board.bitboards[opp_queen]);
        
        let straight_attacks = self.get_rook_attacks(king_sq as usize, occupied_without_sq);
        let straight_attackers = straight_attacks & (board.bitboards[opp_rook] | board.bitboards[opp_queen]);
        
        if original_straight_attackers == 0 && straight_attackers != 0 {
            let target_mask = 1u64 << target_idx;
            if (straight_attacks & target_mask) == 0 {
                return true; // Absolutely pinned away from target
            }
        }

        false
    }

    pub fn get_attackers_mask(&self, board: &Board, white: bool, target_idx: u8, occupied: u64) -> u64 {
        let mut attackers = 0u64;
        let opp_pawn = if white { BLACK_PAWN } else { WHITE_PAWN };
        let opp_knight = if white { BLACK_KNIGHT } else { WHITE_KNIGHT };
        let opp_bishop = if white { BLACK_BISHOP } else { WHITE_BISHOP };
        let opp_rook = if white { BLACK_ROOK } else { WHITE_ROOK };
        let opp_queen = if white { BLACK_QUEEN } else { WHITE_QUEEN };
        let opp_king = if white { BLACK_KING } else { WHITE_KING };

        // Pawns
        let file = target_idx % 8;
        if white {
            if file > 0 && target_idx <= 56 {
                let sq = target_idx + 7;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 {
                    attackers |= 1u64 << sq;
                }
            }
            if file < 7 && target_idx <= 54 {
                let sq = target_idx + 9;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 {
                    attackers |= 1u64 << sq;
                }
            }
        } else {
            if file > 0 && target_idx >= 9 {
                let sq = target_idx - 9;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 {
                    attackers |= 1u64 << sq;
                }
            }
            if file < 7 && target_idx >= 7 {
                let sq = target_idx - 7;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 {
                    attackers |= 1u64 << sq;
                }
            }
        }

        // Knights
        let knight_attacks = KNIGHT_ATTACKS[target_idx as usize];
        attackers |= knight_attacks & board.bitboards[opp_knight];

        // King
        let king_attacks = KING_ATTACKS[target_idx as usize];
        attackers |= king_attacks & board.bitboards[opp_king];

        // Bishop / Queen diagonals
        let diag_attacks = self.get_bishop_attacks(target_idx as usize, occupied);
        attackers |= diag_attacks & (board.bitboards[opp_bishop] | board.bitboards[opp_queen]);

        // Rook / Queen straights
        let straight_attacks = self.get_rook_attacks(target_idx as usize, occupied);
        attackers |= straight_attacks & (board.bitboards[opp_rook] | board.bitboards[opp_queen]);

        attackers
    }

    pub fn get_attackers_mask_for_see(&self, board: &Board, white: bool, target_idx: u8, occupied: u64) -> u64 {
        let mut attackers = 0u64;
        let opp_pawn = if white { BLACK_PAWN } else { WHITE_PAWN };
        let opp_knight = if white { BLACK_KNIGHT } else { WHITE_KNIGHT };
        let opp_bishop = if white { BLACK_BISHOP } else { WHITE_BISHOP };
        let opp_rook = if white { BLACK_ROOK } else { WHITE_ROOK };
        let opp_queen = if white { BLACK_QUEEN } else { WHITE_QUEEN };
        let opp_king = if white { BLACK_KING } else { WHITE_KING };

        // Pawns
        let file = target_idx % 8;
        if white {
            if file > 0 && target_idx <= 56 {
                let sq = target_idx + 7;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 && !self.is_pinned_away_from_target(board, sq, target_idx, false) {
                    attackers |= 1u64 << sq;
                }
            }
            if file < 7 && target_idx <= 54 {
                let sq = target_idx + 9;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 && !self.is_pinned_away_from_target(board, sq, target_idx, false) {
                    attackers |= 1u64 << sq;
                }
            }
        } else {
            if file > 0 && target_idx >= 9 {
                let sq = target_idx - 9;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 && !self.is_pinned_away_from_target(board, sq, target_idx, true) {
                    attackers |= 1u64 << sq;
                }
            }
            if file < 7 && target_idx >= 7 {
                let sq = target_idx - 7;
                if (board.bitboards[opp_pawn] & (1u64 << sq)) != 0 && !self.is_pinned_away_from_target(board, sq, target_idx, true) {
                    attackers |= 1u64 << sq;
                }
            }
        }

        // Knights
        let knight_attacks = KNIGHT_ATTACKS[target_idx as usize];
        let mut knights = knight_attacks & board.bitboards[opp_knight];
        while knights != 0 {
            let sq = knights.trailing_zeros() as u8;
            if !self.is_pinned_away_from_target(board, sq, target_idx, !white) {
                attackers |= 1u64 << sq;
            }
            knights &= knights - 1;
        }

        // King
        let king_attacks = KING_ATTACKS[target_idx as usize];
        attackers |= king_attacks & board.bitboards[opp_king];

        // Bishop / Queen diagonals
        let diag_attacks = self.get_bishop_attacks(target_idx as usize, occupied);
        let mut bishops = diag_attacks & (board.bitboards[opp_bishop] | board.bitboards[opp_queen]);
        while bishops != 0 {
            let sq = bishops.trailing_zeros() as u8;
            if !self.is_pinned_away_from_target(board, sq, target_idx, !white) {
                attackers |= 1u64 << sq;
            }
            bishops &= bishops - 1;
        }

        // Rook / Queen straights
        let straight_attacks = self.get_rook_attacks(target_idx as usize, occupied);
        let mut rooks = straight_attacks & (board.bitboards[opp_rook] | board.bitboards[opp_queen]);
        while rooks != 0 {
            let sq = rooks.trailing_zeros() as u8;
            if !self.is_pinned_away_from_target(board, sq, target_idx, !white) {
                attackers |= 1u64 << sq;
            }
            rooks &= rooks - 1;
        }

        attackers
    }

    /// Checks which OPPONENT pieces attack the given target_idx.
    pub fn get_attack_idx_list(&self, board: &Board, white: bool, target_idx: i32) -> Vec<i32> {
        if target_idx == -1 {
            return Vec::new();
        }
        let attackers_mask = self.get_attackers_mask(board, white, target_idx as u8, board.occupied);
        let mut attackers = Vec::new();
        let mut temp = attackers_mask;
        while temp != 0 {
            let sq = temp.trailing_zeros() as i32;
            attackers.push(sq);
            temp &= temp - 1;
        }
        attackers
    }

    /// Checks if the target index is under shadow attack.
    pub fn get_attack_idx_list_with_shadow(&self, board: &Board, white: bool, target_idx: i32) -> Vec<i32> {
        let mut current_occupied = board.occupied;
        let mut all_attackers = Vec::default();

        let mut attackers_mask = self.get_attackers_mask(board, white, target_idx as u8, current_occupied);
        while attackers_mask != 0 {
            let attacker = attackers_mask.trailing_zeros() as u8;
            all_attackers.push(attacker as i32);
            current_occupied &= !(1u64 << attacker);
            attackers_mask = self.get_attackers_mask(board, white, target_idx as u8, current_occupied);
            for &found in &all_attackers {
                attackers_mask &= !(1u64 << found);
            }
        }
        all_attackers
    }

    /// Checks if the king is under attack.
    pub fn get_check_idx_list(&self, board: &Board, white: bool) -> Vec<i32> {
        let king_pos = if white {
            board.get_king_positions().0
        } else {
            board.get_king_positions().1
        };
        self.get_attack_idx_list(board, white, king_pos)
    }

    /// Returns true if the king of side to move is in check, otherwise false.
    pub fn is_in_check(&self, board: &Board) -> bool {
        self.__check_check(board, false)
    }

    pub fn __check_check(&self, board: &Board, inverse: bool) -> bool {
        let king_positions = board.get_king_positions();
        let white = if inverse {
            !board.white_to_move
        } else {
            board.white_to_move
        };
        let target_idx = if white {
            king_positions.0
        } else {
            king_positions.1
        };
        if target_idx == -1 {
            return false;
        }
        let attackers_mask = self.get_attackers_mask(board, white, target_idx as u8, board.occupied);
        attackers_mask != 0
    }
}

#[cfg(test)]
mod tests {
    use crate::notation_util::NotationUtil;
    use crate::service::Service;
    use crate::zobrist::ZobristTable;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use super::*;

    #[test]
    fn test_get_king_attacks() {
        let movegen = MoveGenService::new();
        let e4_attacks = movegen.get_king_attacks(28);
        assert_eq!(e4_attacks.count_ones(), 8);

        let a1_attacks = movegen.get_king_attacks(0);
        assert_eq!(a1_attacks.count_ones(), 3);
        assert_ne!(a1_attacks & (1u64 << 1), 0);
        assert_ne!(a1_attacks & (1u64 << 8), 0);
        assert_ne!(a1_attacks & (1u64 << 9), 0);
    }

    // Test-only mailbox-to-lerf translator to preserve all test coordinates
    fn m2l(sq: i32) -> i32 {
        if sq < 0 {
            return sq;
        }
        let row = sq / 10;
        let col = sq % 10;
        if row < 2 || row > 9 || col < 1 || col > 8 {
            panic!("Invalid mailbox index: {}", sq);
        }
        (9 - row) * 8 + (col - 1)
    }

    fn generate_valid_moves_list(board: &mut Board) -> Vec<Turn> {
        let service = Service::new();
                let config = Config::for_tests();
        let zobrist_table = ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
        start_time: std::time::Instant::now(),
                            target_time: None,
                            root_moves_total: 0,
                            root_moves_searched: 0,
                            root_depth: 0,
                        };

        let mut move_list = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &context, true, &mut move_list);
        move_list.as_slice().to_vec()
    }

    fn generate_valid_moves_list_capture(board: &mut Board) -> Vec<Turn> {
        let service = Service::new();
                let config = Config::for_tests();
        let zobrist_table = ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
        start_time: std::time::Instant::now(),
                            target_time: None,
                            root_moves_total: 0,
                            root_moves_searched: 0,
                            root_depth: 0,
                        };

        let mut move_list = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list_capture(board, &mut Stats::new(), &config, &context, true, &mut move_list);
        move_list.as_slice().to_vec()
    }

    #[test]
    fn get_check_idx_list_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        // Test 1: Initial Board Setup - No Check
        let mut board = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert!(move_gen_service.get_check_idx_list(&board, board.white_to_move).is_empty());

        // Test 2: Scenario where check occurs
        board = fen_service.set_fen("r1bqk1nr/pppp2pp/4p3/8/1b3P2/3PPn2/PPP2pPP/RNBQKBNR w KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert!(check_idx_list.contains(&m2l(86)), "Check index list should contain f2");
        assert!(check_idx_list.contains(&m2l(76)), "Check index list should contain f3");
        assert!(check_idx_list.contains(&m2l(62)), "Check index list should contain b4");

        // Test 3: Black turn - No check
        board = fen_service.set_fen("r1bqk1nr/pppp2pp/4p3/8/1b3P2/3PPn2/PPP2pPP/RNBQKBNR b KQkq - 0 1");
        assert!(move_gen_service.get_check_idx_list(&board, board.white_to_move).is_empty());

        // Test 4: Two checks, positions 36 and 37 (mapped to mailbox)
        board = fen_service.set_fen("r1bqk1nr/pppp1PNp/4p3/1Q5B/1b3P2/3PPn2/PPP2p1P/RN2KB1R b KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert_eq!(check_idx_list.len(), 2);
        assert!(check_idx_list.contains(&m2l(36)), "Check index list should contain f7");
        assert!(check_idx_list.contains(&m2l(37)), "Check index list should contain g7");

        // Test 5: Four checks in various positions
        board = fen_service.set_fen("r1Rqk2r/pppP2Np/5n2/3p3B/1b2QP2/3PPn2/PPP2p1P/RN2KB2 b KQkq - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert_eq!(check_idx_list.len(), 4);
        assert!(check_idx_list.contains(&m2l(37)), "Check index list should contain g7");
        assert!(check_idx_list.contains(&m2l(58)), "Check index list should contain h5");
        assert!(check_idx_list.contains(&m2l(65)), "Check index list should contain e4");
        assert!(check_idx_list.contains(&m2l(34)), "Check index list should contain d7");

        // Test 6: Four checks in another scenario
        board = fen_service.set_fen("2B5/6N1/4k3/8/2K2NP1/1B2Q2B/4R3/8 b - - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert!(check_idx_list.contains(&m2l(23)), "Check index list should contain c4");
        assert!(check_idx_list.contains(&m2l(37)), "Check index list should contain g4");
        assert!(check_idx_list.contains(&m2l(66)), "Check index list should contain f4");
        assert!(check_idx_list.contains(&m2l(75)), "Check index list should contain e3");
        assert_eq!(check_idx_list.len(), 4);

        // Test 7: Last check scenario with two checks
        board = fen_service.set_fen("8/1k6/8/1q6/2b1r3/8/1rn1K3/8 w - - 0 1");
        let check_idx_list = move_gen_service.get_check_idx_list(&board, board.white_to_move);
        assert!(check_idx_list.contains(&m2l(63)), "Check index list should contain c4");
        assert!(check_idx_list.contains(&m2l(65)), "Check index list should contain e4");
        assert_eq!(check_idx_list.len(), 2);
    }

    #[test]
    fn get_attack_idx_list_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        let board = fen_service.set_fen("r1q2r1k/1pp1bpp1/p2p1n2/4P2p/2Q2B2/2N4P/PPPR1PP1/3R2K1 b - - 3 16");
        let attacks = move_gen_service.get_attack_idx_list(&board, board.white_to_move, m2l(44)).len() as i32;
        assert_eq!(2, attacks);

        let board = fen_service.set_fen("r1bqnr2/pp1nbpk1/2p1p3/3p2pp/2PP1P1N/2NBP1B1/PPQ3PP/2R1K2R w K - 0 14");
        let attacks = move_gen_service.get_attack_idx_list(&board, board.white_to_move, m2l(68)).len() as i32;
        assert_eq!(1, attacks);

        let board = fen_service.set_fen("r2qkb1r/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
        let attacks = move_gen_service.get_attack_idx_list(&board, board.white_to_move, m2l(24)).len() as i32;
        assert_eq!(0, attacks);
    }

    #[test]
    fn get_attack_idx_list_with_shadow_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        let board = fen_service.set_fen("r1q2r1k/1pp1bpp1/p2p1n2/4P2p/2Q2B2/2N4P/PPPR1PP1/3R2K1 b - - 3 16");
        let attacks = move_gen_service.get_attack_idx_list_with_shadow(&board, board.white_to_move, m2l(44)).len() as i32;
        assert_eq!(4, attacks);

        let board = fen_service.set_fen("r1bqnr2/pp1nbpk1/2p1p3/3p2pp/2PP1P1N/2NBP1B1/PPQ3PP/2R1K2R w K - 0 14");
        let attacks = move_gen_service.get_attack_idx_list_with_shadow(&board, board.white_to_move, m2l(68)).len() as i32;
        assert_eq!(3, attacks);
    }

    #[test]
    fn generate_moves_list_for_fen_test() {
        let fen_service = Service::new().fen;
        let move_gen_service = Service::new().move_gen;

        // Test: Standard starting position
        let board = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let mut raw_moves = crate::model::MoveRawList::new();
        let masks = move_gen_service.compute_node_masks(&board);
        move_gen_service.generate_moves_list_for_piece(&board, 0, false, &masks, &mut raw_moves);
        let moves: Vec<i32> = raw_moves.moves[0..raw_moves.len].iter().map(|&x| x as i32).collect();

        let expected_moves_mailbox = vec![
            81, 71, 81, 61, 82, 72, 82, 62, 83, 73, 83, 63, 84, 74, 84, 64,
            85, 75, 85, 65, 86, 76, 86, 66, 87, 77, 87, 67, 88, 78, 88, 68,
            92, 71, 92, 73, 97, 76, 97, 78
        ];
        let expected_moves: Vec<i32> = expected_moves_mailbox.into_iter().map(m2l).collect();
        // Since the bitboard move list ordering might differ from mailbox ordering, we sort them before asserting
        let mut sorted_moves = moves.clone();
        let mut sorted_expected = expected_moves.clone();
        sorted_moves.sort();
        sorted_expected.sort();
        assert_eq!(sorted_moves, sorted_expected, "Move Gen for start-up setup is wrong");

        // Test: White in check and only a few moves are available for the king
        let board = fen_service.set_fen("rnbqk2r/pppp1ppp/4p3/8/1b6/3P1n1B/PPP1PPPP/RNBQK1NR w KQkq - 0 1");
        let mut raw_moves_in_check = crate::model::MoveRawList::new();
        let masks = move_gen_service.compute_node_masks(&board);
        move_gen_service.generate_moves_list_for_piece(&board, 0, false, &masks, &mut raw_moves_in_check);
        let moves_in_check: Vec<i32> = raw_moves_in_check.moves[0..raw_moves_in_check.len].iter().map(|&x| x as i32).collect();

        // Double check from the b4 bishop and the f3 knight, so only the king may move. e1-d2 is
        // covered by both checkers and never appears: raw generation is legal by construction and
        // no longer emits king moves onto attacked squares.
        let expected_moves_in_check_mailbox = vec![95, 96];
        let mut expected_moves_in_check: Vec<i32> = expected_moves_in_check_mailbox.into_iter().map(m2l).collect();
        let mut sorted_moves_in_check = moves_in_check.clone();
        sorted_moves_in_check.sort();
        expected_moves_in_check.sort();
        assert_eq!(sorted_moves_in_check, expected_moves_in_check, "Check list is not working");
    }

    #[test]
    fn get_valid_moves_from_move_list_test_double_check() {
        let service = &Service::new();

        // Double check, only one king move is possible for white
        let mut board = service.fen.set_fen("rnbqk2r/pppp1ppp/4p3/8/1b6/3P1n1B/PPP1PPPP/RNBQK1NR w KQkq - 0 1");
        let valid_turn_list = generate_valid_moves_list(&mut board);
        assert_eq!(valid_turn_list.len(), 1);
        assert!(valid_turn_list[0].from == m2l(95) as u8 && valid_turn_list[0].to == m2l(96) as u8);

        // Double check, only one king move is possible for black
        let mut board = service.fen.set_fen("rnb1k1nr/ppppp1Np/2N5/7Q/8/4P3/PPPP1PPP/RNB1KB1R b KQkq - 0 1");
        let valid_turn_list = generate_valid_moves_list(&mut board);
        assert_eq!(valid_turn_list.len(), 1);
        assert!(valid_turn_list[0].from == m2l(25) as u8 && valid_turn_list[0].to == m2l(26) as u8);
    }

    #[test]
    fn get_valid_moves_when_in_check_easy() {
        test_fen("rnbqk1nr/pppp1ppp/4p3/8/1b6/3P1P2/PPP1P1PP/RNBQKBNR w KQkq - 1 3", 6);
        test_fen("rnbqkb1r/pppppppp/8/8/8/5n2/PPPPQ1PP/RNB1KBNR w KQkq - 0 1", 5);
        test_fen("8/5k2/3r4/5n2/3N4/3K4/1q6/8 w - - 0 1", 2);
        test_fen("8/5k2/3r4/5n2/8/3K1N2/1q6/8 w - - 0 1", 3);

        test_fen("rnbqkbnr/ppp1pppp/3p4/1B6/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 1 2", 5);
        test_fen("r1bqkbnr/pp1npppp/2Bp4/8/4P3/3P4/PPP2PPP/RNBQK1NR b KQkq - 0 4", 20);
        test_fen("rnbqk2r/pppp2pp/5p1n/2b1p2Q/2B1P3/P6N/1PPP1PPP/RNB1K2R b KQkq - 1 5", 4);
        test_fen("8/8/3k1N1p/3b4/3Q4/8/4R3/3K4 b - - 0 1", 3);
    }

    #[test]
    fn castling_test() {
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1", 25, "e1g1");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1", 25, "e8g8");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w Kk - 0 1", 24, "e1g1");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b Kk - 0 1", 24, "e8g8");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w Qq - 0 1", 24, "e1c1");
        test_fen_with_move("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b Qq - 0 1", 24, "e8c8");
        test_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w - - 0 1", 23);
        test_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b - - 0 1", 23);
        test_fen("r3k2r/pppppppp/8/8/8/1n4n1/PPPPPPPP/R3K2R w KQkq - 0 1", 22);
        test_fen("r3k2r/pppppppp/1N4N1/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1", 22);
    }

    #[test]
    fn promotion_test() {
        test_fen_with_move("5n2/4P3/8/2k5/8/8/2K5/8 w - - 0 1", 16, "e7f8q");

        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("5n2/4P3/8/2k5/8/8/2K5/8 w - - 0 1");
        let board_copy = board.clone();

        let mut promotion_move = NotationUtil::get_turn_from_notation("e7f8q");
        promotion_move.capture = 22;
        let move_info = board.do_move(&promotion_move);

        assert_eq!(board.get_piece_at(m2l(26) as u8), 14, "White promotion should result in a queen (14)");
        board.undo_move(&promotion_move, move_info);
        assert_eq!(board.get_piece_at(m2l(26) as u8), 22, "Piece should revert to captured knight (22)");
        assert_eq!(board.get_piece_at(m2l(35) as u8), 10, "White pawn should be back at e7");
        assert_eq!(board, board_copy, "Board should be restored");

        let mut board = fen_service.set_fen("8/2p2p1p/p4kp1/1r6/8/K7/3p4/8 b - - 1 51");
        let turns = generate_valid_moves_list(&mut board);
        let promotion_move = turns.iter().find(|t| t.is_promotion()).expect("No Promotion move found");
        assert_eq!(24, promotion_move.promotion);
        let mi = board.do_move(promotion_move);
        assert_eq!(24, board.get_piece_at(m2l(94) as u8));
        assert_eq!(0, board.get_piece_at(m2l(84) as u8));

        board.undo_move(promotion_move, mi);
        assert_eq!(0, board.get_piece_at(m2l(94) as u8));
        assert_eq!(20, board.get_piece_at(m2l(84) as u8));

        let move_from_notation_util = NotationUtil::get_turn_from_notation("d2d1q");
        board.do_move(&move_from_notation_util);
        assert_eq!(24, board.get_piece_at(m2l(94) as u8));
        assert_eq!(0, board.get_piece_at(m2l(84) as u8));

        let move_list = test_fen_with_move("8/2k1P3/8/7b/8/4b3/2K3n1/8 w - - 0 1", 9, "e7e8n");
        assert_eq!(move_list.len(), 8, "Expected 8 moves after knight promotion");

        let move_list = test_fen_with_move("8/2k1P3/8/7b/8/4b3/2K3n1/8 w - - 0 1", 9, "e7e8q");
        assert_eq!(move_list.len(), 24, "Expected 24 moves after queen promotion");

        let move_list = test_fen_with_move("5k2/R6P/8/8/8/2K5/8/6r1 w - - 0 1", 25, "h7h8q");
        assert_eq!(move_list.len(), 1, "Expected 1 move after queen promotion on h8");

        let move_list = test_fen_with_move("8/8/8/8/8/1K6/5p2/4k3 b - - 0 1", 8, "f2f1q");
        assert_eq!(move_list.len(), 7, "Expected 7 moves after black queen promotion");

        let move_list = test_fen_with_move("8/8/8/8/8/1K6/5p2/4k3 b - - 0 1", 8, "f2f1n");
        assert_eq!(move_list.len(), 8, "Expected 8 moves after black knight promotion");

        let move_list = test_fen_with_move("8/8/3k4/8/8/8/1K5p/8 b - - 0 1", 12, "h2h1q");
        assert_eq!(move_list.len(), 5, "Expected 5 moves after black queen promotion on h1");
    }

    #[test]
    fn move_list_sort_test() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("r1bqk2r/pppp1ppp/2n2n2/2b5/2BpP3/2P2N2/PP3PPP/RNBQK2R w KQkq - 0 6");
        let move_list = generate_valid_moves_list(&mut board);
        assert!(move_list.get(0).unwrap().from == m2l(63) as u8 && move_list.get(0).unwrap().to == m2l(36) as u8);
        assert!(move_list.last().unwrap().rank >> crate::model::RANK_TIEBREAK_BITS == 0, "the worst move ranks zero above the tie-break lane");

        let mut board = fen_service.set_fen("r1bqr1k1/ppp2pp1/2n2n1p/2bp4/2B1PB2/1NP4P/PP3PP1/RN1Q1RK1 b - - 1 10");
        let move_list = generate_valid_moves_list(&mut board);
        assert!(move_list.get(0).unwrap().from == m2l(53) as u8 && move_list.get(0).unwrap().to == m2l(86) as u8);
        assert!(move_list.get(1).unwrap().from == m2l(54) as u8 && move_list.get(1).unwrap().to == m2l(63) as u8);
        assert!(move_list.last().unwrap().rank >> crate::model::RANK_TIEBREAK_BITS == 0, "the worst move ranks zero above the tie-break lane");
    }

    #[test]
    fn a_reused_move_list_generates_exactly_what_a_new_one_does_test() {
        // The generators append rather than overwrite, and the search now hands one list to
        // every node at a recursion level. The `clear()` at the call site is the only thing that
        // keeps a node from inheriting the moves of the node before it.
        let service = Service::new();
        let config = Config::for_tests();
        let zobrist_table = ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = Mutex::new(HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
            start_time: std::time::Instant::now(),
            target_time: None,
            root_moves_total: 0,
            root_moves_searched: 0,
            root_depth: 0,
        };

        let mut first = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let mut second = service.fen.set_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");

        let mut reused = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(
            &mut first, &mut Stats::new(), &config, &context, true, &mut reused);
        assert!(reused.len > 0, "the first position has moves to inherit");

        reused.clear();
        service.move_gen.generate_valid_moves_list(
            &mut second, &mut Stats::new(), &config, &context, true, &mut reused);

        let fresh = generate_valid_moves_list(&mut second);
        assert_eq!(reused.as_slice().to_vec(), fresh);
    }

    #[test]
    fn game_status_check_mate_test() {
        let board = test_fen("rnbqkbnr/ppppp2p/8/7B/8/8/PPPPPPPP/RNBQK1NR b KQkq - 0 1", 0);
        assert!(board.game_status == GameStatus::WhiteWin);

        let board = test_fen("3N3B/8/6k1/4K3/8/6RR/8/8 b - - 0 1", 0);
        assert!(board.game_status == GameStatus::WhiteWin);

        let board = test_fen("rn2k1nr/pppppppp/3b4/1b6/4P1PN/1B6/PPPP1P1q/RNBQR1K1 w Qkq - 0 1", 0);
        assert!(board.game_status == GameStatus::BlackWin);

        let board = test_fen("4r3/8/8/8/b4n1b/4p3/1k1K4/8 w - - 0 1", 0);
        assert!(board.game_status == GameStatus::BlackWin);

        let board = test_fen("3R3k/6pp/8/8/4P3/8/6PP/7K b - - 0 1", 0);
        assert!(board.game_status == GameStatus::WhiteWin);
    }

    #[test]
    fn game_status_pat_test() {
        let board = test_fen("3N3B/8/6k1/4K3/5P2/7R/8/8 b - - 0 1", 0);
        assert!(board.game_status == GameStatus::Draw);

        let board = test_fen("R7/R2pk3/Q5P1/8/8/8/4K3/8 b - - 0 1", 0);
        assert!(board.game_status == GameStatus::Draw);

        let board = test_fen("8/8/8/8/4k3/2p1n1p1/4K1n1/8 w - - 0 1", 0);
        assert!(board.game_status == GameStatus::Draw);

        let board = test_fen("8/8/8/8/3k2p1/8/r2PKN1r/r7 w - - 0 1", 0);
        assert!(board.game_status == GameStatus::Draw);
    }

    #[test]
    fn get_check_idx_list() {
        test_fen("8/1P4k1/1K5p/4p2P/4r3/8/8/6q1 w - - 0 59", 5);
    }

    #[test]
    fn hit_moves_count_and_undo_test() {
        let fen_service = Service::new().fen;

        let mut board = fen_service.set_fen("r1bqr1k1/2p2ppp/p1np1n2/1pb1p1N1/2BPP3/2P1B3/PPQ2PPP/RN3RK1 w - - 0 10");
        let capture_moves = generate_valid_moves_list_capture(&mut board);
        assert_eq!(6, capture_moves.len());
        let copy_board = board.clone();
        let capture_move = capture_moves.get(0).unwrap();
        let mi = board.do_move(capture_move);
        board.undo_move(capture_move, mi);
        assert_eq!(copy_board, board);

        let mut board = fen_service.set_fen("r1bqr1k1/2p2ppp/p1np1n2/1pb1p1N1/2BPP3/2P1B3/PPQ2PPP/RN3RK1 b - - 0 10");
        let capture_moves = generate_valid_moves_list_capture(&mut board);
        assert_eq!(5, capture_moves.len());
        let copy_board = board.clone();
        let capture_move = capture_moves.get(0).unwrap();
        let mi = board.do_move(capture_move);
        board.undo_move(capture_move, mi);
        assert_eq!(copy_board, board);
    }

    #[test]
    fn en_passante_test() {
        let fen_service = Service::new().fen;

        let board = fen_service.set_fen("rnbqkbnr/ppp1ppp1/8/3pP2p/8/7P/PPPP1PP1/RNBQKBNR w KQkq d6 0 4");
        assert_eq!(m2l(44) as i8, board.field_for_en_passante);

        let board = fen_service.set_fen("rnbqkbnr/ppp1pppp/8/8/3pP2P/6P1/PPPP1P2/RNBQKBNR b KQkq e3 0 3");
        assert_eq!(m2l(75) as i8, board.field_for_en_passante);

        let truncate = Config::new().truncate_bad_moves;

        test_fen_with_move("rnbqkbnr/pp1ppp2/7p/1PpP2p1/8/8/P1P1PPPP/RNBQKBNR w KQkq c6 0 5", 31.min(truncate), "b5c6");
        test_fen_with_move("rnbqkbnr/pp1ppp2/7p/1PpP2p1/8/8/P1P1PPPP/RNBQKBNR w KQkq c6 0 5", 31.min(truncate), "d5c6");
        test_fen("rnbqkbnr/pp1ppp2/7p/1PpP2p1/8/8/P1P1PPPP/RNBQKBNR w - KQkq 0 5", 29);

        test_fen_with_move("rnbqkbnr/ppp1p1pp/8/8/3pPp1P/PP6/2PP1PP1/RNBQKBNR b KQkq e3 0 5", 31.min(truncate), "d4e3");
        test_fen_with_move("rnbqkbnr/ppp1p1pp/8/8/3pPp1P/PP6/2PP1PP1/RNBQKBNR b KQkq e3 0 5", 31.min(truncate), "f4e3");
        test_fen("rnbqkbnr/ppp1p1pp/8/8/3pPp1P/PP6/2PP1PP1/RNBQKBNR b KQkq - 0 5", 29);
    }

    #[test]
    fn check_moves_when_in_check() {
        let service = Service::new();
        let board = &mut service.fen.set_fen("7r/p1p2p1p/P3k1p1/2K2r2/2P5/8/8/8 w - - 0 36");
        let turns = generate_valid_moves_list(board);
        assert_eq!(3, turns.len());
    }

    #[test]
    fn move_ordering_with_pv_nodes_test() {
        let service = Service::new();
        let config = Config::for_tests();
        
        let board = &mut service.fen.set_init_board();

        let mut move_row = Vec::default();
        move_row.push(Turn::_new_to_from(m2l(81) as u8, m2l(61) as u8));
        move_row.push(Turn::_new_to_from(m2l(38) as u8, m2l(58) as u8));

        let mut pv_nodes_map = HashMap::new();
        let old_board = board.clone();
        for turn in &move_row {
            let hash = zobrist::gen_hash(board);
            pv_nodes_map.insert(hash, *turn);
            board.do_move(turn);
        }
        *board = old_board;
        let pv_nodes = Mutex::new(pv_nodes_map);

        let zobrist_table = ZobristTable::with_capacity(1_000);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
        start_time: std::time::Instant::now(),
                            target_time: None,
                            root_moves_total: 0,
                            root_moves_searched: 0,
                            root_depth: 0,
                        };

        let mut move_list = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &context, true, &mut move_list);
        let turns = move_list.as_slice().to_vec();
        let first_turn = turns.get(0).unwrap();

        assert_eq!(m2l(81) as u8, first_turn.from);
        assert_eq!(m2l(61) as u8, first_turn.to);
    }

    #[test]
    fn skip_validation_and_check_game_end_test() {
        let service = Service::new();
        let config = Config::for_tests();
        
        let zobrist_table = ZobristTable::with_capacity(1_000);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
        start_time: std::time::Instant::now(),
                            target_time: None,
                            root_moves_total: 0,
                            root_moves_searched: 0,
                            root_depth: 0,
                        };

        let board = &mut service.fen.set_fen("r1bqk1nr/ppp2ppp/2P5/4p3/2B5/3P1N2/PPP2PPP/RNBQb2R w kq - 0 1");

        
        let mut move_list = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &context, true, &mut move_list);
        let turns = move_list.as_slice().to_vec();
        assert_eq!(38, turns.len());

        let board = &mut service.fen.set_fen("r1bqk1nr/ppp2ppp/2P5/4p3/1bB5/3P1N2/PPP2PPP/RNBQK2R b KQkq - 0 1");
        let turn = Turn::new(m2l(62) as u8, m2l(95) as u8, 15, 0, false, 0);
        let mi = board.do_move(&turn);
        assert_eq!(false, board._white_king_on_board);
        assert_eq!(true, board._black_king_on_board);
        assert_eq!(GameStatus::BlackWin, board.game_status);
        let mut move_list = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(board, &mut Stats::new(), &config, &context, true, &mut move_list);
        let turns = move_list.as_slice().to_vec();
        assert_eq!(0, turns.len());
        board.undo_move(&turn, mi);
        assert_eq!(true, board._white_king_on_board);
        assert_eq!(true, board._black_king_on_board);
        assert_eq!(GameStatus::Normal, board.game_status);

        let board = &mut service.fen.set_fen("r2qk1nr/pPp2ppp/8/4p3/Qbb5/2PP1N2/PP3PPP/RNB1K2R w KQkq - 0 1");
        let turn = Turn::new(m2l(61) as u8, m2l(25) as u8, 25, 0, false, 0);
        let mi = board.do_move(&turn);
        assert_eq!(true, board._white_king_on_board);
        assert_eq!(false, board._black_king_on_board);
        assert_eq!(GameStatus::WhiteWin, board.game_status);
        assert_eq!(0, turns.len());
        board.undo_move(&turn, mi);
        assert_eq!(true, board._white_king_on_board);
        assert_eq!(true, board._black_king_on_board);
        assert_eq!(GameStatus::Normal, board.game_status);
    }

    #[test]
    fn is_in_check_test() {
        let fen = Service::new().fen;
        let movegen = Service::new().move_gen;

        let board = fen.set_fen("r1b1kbnr/1pp1qppp/p1n5/3Pp3/B7/5N2/PPPP1PPP/RNBQK2R w KQkq - 1 6");
        assert!(!movegen.is_in_check(&board));

        let board = fen.set_fen("r1b1kbnr/1pp2ppp/p1n5/3Pq3/B7/8/PPPP1PPP/RNBQK2R w KQkq - 0 7");
        assert!(movegen.is_in_check(&board));

        let board = fen.set_fen("r1b1k1nr/1pp2ppp/p1B5/3Pq3/1b6/8/PPPPQPPP/RNB1K2R b KQkq - 0 8");
        assert!(movegen.is_in_check(&board));

        let board = fen.set_fen("r1bk2nr/1pp3pp/2B1Np2/p2Pq3/8/b7/PPPPQPPP/R1B1K2R b KQ - 1 12");
        assert!(movegen.is_in_check(&board));

        let board = fen.set_fen("r1b4r/1pp1k1pp/2B1Np2/p2Pq3/8/b4nP1/PPPPQP2/R1B2RK1 w - - 1 17");
        assert!(movegen.is_in_check(&board));

        let board = fen.set_fen("r1b4r/1pp1k1pp/2B1Np2/p2P2q1/8/b4QP1/PPPP1P2/R1B2RK1 w - - 1 18");
        assert!(!movegen.is_in_check(&board));

        let board = fen.set_fen("r1b4r/1pp1k1pp/2B1Np2/p2P2q1/4Q3/b5P1/PPPP1P2/R1B2RK1 b - - 2 18");
        assert!(!movegen.is_in_check(&board));
    }

    fn test_fen(fen: &str, allowed_moves: usize) -> Board {
        let fen_service = Service::new().fen;
        let mut board = fen_service.set_fen(fen);
        let moves = generate_valid_moves_list(&mut board);
        assert_eq!(moves.len(), allowed_moves, "Expected {} moves, but got {} for FEN: {}", allowed_moves, moves.len(), fen);
        board
    }

    fn test_fen_with_move(fen: &str, allowed_moves: usize, notation: &str) -> Vec<Turn> {
        let mut board = test_fen(fen, allowed_moves);
        let board_copy = board.clone();
        let move_list = generate_valid_moves_list(&mut board);
        let move_turn = NotationUtil::_get_turn_from_list(&move_list, notation);
        let move_info = board.do_move(&move_turn);
        let opponent_moves = generate_valid_moves_list(&mut board);
        board.undo_move(&move_turn, move_info);
        assert_eq!(&board, &board_copy, "Board should be restored after undoing the move");
        opponent_moves
    }

    /// Walks the complete move tree and verifies the generator against the definition of
    /// legality and check that it replaced: play the move, then look at whether the mover left
    /// its own king attacked and whether the enemy king ends up attacked.
    ///
    /// This is the exhaustive form of the `do_move`/`undo_move` validation the generator
    /// performed inline until v0.31.0, and it is the load-bearing test for this change. Perft
    /// node counts alone cannot catch a wrong `gives_check`, because the flag does not alter the
    /// number of nodes — but it does steer five pruning stages in the search.
    fn perft_verified(movegen: &MoveGenService, board: &mut Board, depth: usize, config: &Config, context: &SearchContext) -> u64 {
        if depth == 0 {
            return 1;
        }

        let mut move_list = crate::model::MoveList::new();
        movegen.generate_valid_moves_list(board, &mut Stats::new(), config, context, true, &mut move_list);

        let mut nodes = 0;
        for i in 0..move_list.len {
            let turn = move_list.moves[i];
            let move_info = board.do_move(&turn);

            let (white_king, black_king) = board.get_king_positions();
            let mover_is_white = !board.white_to_move;
            let own_king = if mover_is_white { white_king } else { black_king };
            let opp_king = if mover_is_white { black_king } else { white_king };

            assert!(
                own_king == -1
                    || movegen.get_attackers_mask(board, mover_is_white, own_king as u8, board.occupied) == 0,
                "illegal move {} accepted by the generator",
                turn.to_algebraic()
            );

            let expected_check = opp_king != -1
                && movegen.get_attackers_mask(board, !mover_is_white, opp_king as u8, board.occupied) != 0;
            assert_eq!(
                turn.gives_check,
                expected_check,
                "wrong gives_check on {}",
                turn.to_algebraic()
            );

            nodes += perft_verified(movegen, board, depth - 1, config, context);
            board.undo_move(&turn, move_info);
        }

        nodes
    }

    /// Runs `perft_verified` over a list of `(fen, depth, expected_nodes)` cases.
    fn run_perft_suite(cases: &[(&str, usize, u64)]) {
        let service = Service::new();
        let mut config = Config::for_tests();
        // Perft counts every legal move; the search-time move cap would silently drop some.
        config.truncate_bad_moves = 256;

        let zobrist_table = ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
            start_time: std::time::Instant::now(),
            target_time: None,
            root_moves_total: 0,
            root_moves_searched: 0,
            root_depth: 0,
        };

        for &(fen, depth, expected) in cases {
            let mut board = service.fen.set_fen(fen);
            let start = std::time::Instant::now();
            let nodes = perft_verified(&service.move_gen, &mut board, depth, &config, &context);
            assert_eq!(nodes, expected, "perft({}) mismatch for {}", depth, fen);
            println!("perft({}) = {:>10} in {:>8.3?}  {}", depth, nodes, start.elapsed(), fen);
        }
    }

    /// The six standard perft positions, at depths that keep the default suite fast. Every move
    /// generated along the way is checked for legality and for a correct `gives_check` flag.
    #[test]
    fn perft_verified_standard_positions_test() {
        run_perft_suite(&[
            ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 3, 8902),
            ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 3, 97862),
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 4, 43238),
            ("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", 3, 9467),
            ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 3, 62379),
            ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", 2, 2079),
        ]);
    }

    /// The full sweep. Slow, and mandatory before a release: run with `cargo test -- --ignored`.
    ///
    /// The first block is the six standard positions at full depth. The second is the published
    /// TalkChess special-case suite, which targets exactly the move types that the pin and check
    /// masks cannot settle on their own — en passant that is pinned along a rank or that answers
    /// a check, castling into or out of an attacked square, and promotions that give check.
    #[test]
    #[ignore]
    fn perft_verified_deep_sweep_test() {
        run_perft_suite(&[
            ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 5, 4865609),
            ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 4, 4085603),
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 6, 11030083),
            ("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", 5, 15833292),
            ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 4, 2103487),
            ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", 4, 3894594),

            // Illegal en passant: the capture would expose the king along the rank.
            ("3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1", 6, 1134888),
            ("8/8/4k3/8/2p5/8/B2P2K1/8 w - - 0 1", 6, 1015133),
            // En passant that checks the opponent.
            ("8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1", 6, 1440467),
            // Castling that gives check, on both wings.
            ("5k2/8/8/8/8/8/8/4K2R w K - 0 1", 6, 661072),
            ("3k4/8/8/8/8/8/8/R3K3 w Q - 0 1", 6, 803711),
            // Castling rights and castling prevented by attacked transit squares.
            ("r3k2r/1b4bq/8/8/8/8/7B/R3K2R w KQkq - 0 1", 4, 1274206),
            ("r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1", 4, 1720476),
            // Promotion out of check, and promotions that give check.
            ("2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1", 6, 3821001),
            ("8/8/1P2K3/8/2n5/1q6/8/5k2 b - - 0 1", 5, 1004658),
            ("4k3/1P6/8/8/8/8/K7/8 w - - 0 1", 6, 217342),
            ("8/P1k5/K7/8/8/8/8/8 w - - 0 1", 6, 92683),
            // Stalemate and checkmate detection at the leaves.
            ("K1k5/8/P7/8/8/8/8/8 w - - 0 1", 6, 2217),
            ("8/k1P5/8/1K6/8/8/8/8 w - - 0 1", 7, 567584),
            ("8/8/2k5/5q2/5n2/8/5K2/8 b - - 0 1", 4, 23527),
        ]);
    }
    #[test]
    fn underpromotions_config_test() {
        let service = Service::new();
        let fen_service = service.fen;
        let mut board = fen_service.set_fen("7k/P7/8/8/8/8/8/7K w - - 0 1");

        let mut config = Config::for_tests();
        config.use_underpromotions = true;
        
                let zobrist_table = ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
        start_time: std::time::Instant::now(),
                            target_time: None,
                            root_moves_total: 0,
                            root_moves_searched: 0,
                            root_depth: 0,
                        };

        let mut move_list = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(&mut board, &mut Stats::new(), &config, &context, true, &mut move_list);
        
        let mut promotions = vec![];
        for turn in move_list.as_slice() {
            if turn.promotion != 0 {
                promotions.push(turn.promotion);
            }
        }
        promotions.sort();
        assert_eq!(promotions, vec![11, 12, 13, 14]);

        config.use_underpromotions = false;
        let mut move_list_disabled = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(&mut board, &mut Stats::new(), &config, &context, true, &mut move_list_disabled);

        let mut promotions_disabled = vec![];
        for turn in move_list_disabled.as_slice() {
            if turn.promotion != 0 {
                promotions_disabled.push(turn.promotion);
            }
        }
        promotions_disabled.sort();
        assert_eq!(promotions_disabled, vec![12, 14]);
    }

    #[test]
    fn obvious_move_early_exit_test() {
        let service = Service::new();
        let fen_service = service.fen;
        let move_gen_service = service.move_gen;
        let config = Config::for_tests();
        let zobrist_table = crate::zobrist::ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = crate::model::SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
        start_time: std::time::Instant::now(),
                            target_time: None,
                            root_moves_total: 0,
                            root_moves_searched: 0,
                            root_depth: 0,
                        };
        

        let mut board = fen_service.set_fen("6k1/8/8/2b5/8/8/6PP/5RqK w - - 0 1");
        
        let mut move_list = crate::model::MoveList::new();
        move_gen_service.generate_valid_moves_list(&mut board, &mut Stats::new(), &config, &context, true, &mut move_list);

        assert_eq!(move_list.len, 1, "Expected exactly 1 legal move in this position!");
        let mv = move_list.as_slice()[0].to_algebraic();
        assert_eq!(mv, "f1g1", "Expected the only legal move to be f1g1!");
    }
    #[test]
    fn test_my_bug_fen() {
        // e4h4 should be generated
        let mut board = test_fen("r1b1k2r/ppp1b1pp/2p5/4N3/3PR3/6Pq/PPP4P/R1BQ2K1 w kq - 1 15", 38);
        let moves = generate_valid_moves_list(&mut board);
        let mut found = false;
        for m in moves {
            if m.to_algebraic() == "e4h4" {
                found = true;
            }
        }
        assert!(found, "e4h4 is not generated");
    }

    #[test]
    fn test_pawn_key_consistency() {
        let service = Service::new();
        let fens = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r1bqk2r/pp2bppp/2n1pn2/2pp2B1/3P4/2N1PN2/PPP1BPPP/R2QK2R w KQkq - 2 7"
        ];

        fn check_pawn_key_recursive(board: &mut Board, service: &Service, depth: i32) {
            let expected_key = crate::zobrist::gen_pawn_hash(board);
            assert_eq!(board.pawn_key, expected_key, "Pawn key mismatch at FEN: {}", service.fen.get_fen(board));

            if depth == 0 {
                return;
            }

            let moves = generate_valid_moves_list(board);
            for m in &moves {
                let move_info = board.do_move(m);
                check_pawn_key_recursive(board, service, depth - 1);
                board.undo_move(m, move_info);
                let expected_key_after_undo = crate::zobrist::gen_pawn_hash(board);
                assert_eq!(board.pawn_key, expected_key_after_undo, "Pawn key mismatch after undo of move {} at FEN: {}", m.to_algebraic(), service.fen.get_fen(board));
            }
        }

        for fen in fens {
            let mut board = service.fen.set_fen(fen);
            check_pawn_key_recursive(&mut board, &service, 2);
        }
    }

    #[test]
    fn test_generate_moves_list_bitboard_consistency() {
        let service = Service::new();
        let fens = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r1bqk2r/pp2bppp/2n1pn2/2pp2B1/3P4/2N1PN2/PPP1BPPP/R2QK2R w KQkq - 2 7"
        ];
        for fen in fens {
            let board = service.fen.set_fen(fen);
            let mut raw_moves = crate::model::MoveRawList::new();
            let masks = service.move_gen.compute_node_masks(&board);
            service.move_gen.generate_moves_list_for_piece(&board, 0, false, &masks, &mut raw_moves);
            assert!(raw_moves.len > 0, "Move generation produced no moves for FEN: {}", fen);
            assert_eq!(raw_moves.len % 2, 0, "Move list length must be even (from/to pairs)");
        }
    }

    #[test]
    fn test_king_attacks_proximity_masking() {
        let service = Service::new();
        // Kings on e4 (28) and e6 (44). White King on e4 should not be allowed to move to e5 (36), d5 (35), or f5 (37).
        let board = service.fen.set_fen("8/8/4k3/8/4K3/8/8/8 w - - 0 1");
        let mut raw_moves = crate::model::MoveRawList::new();
        let masks = service.move_gen.compute_node_masks(&board);
        service.move_gen.generate_moves_list_for_piece(&board, 0, false, &masks, &mut raw_moves);
        for i in (0..raw_moves.len).step_by(2) {
            let from = raw_moves.moves[i];
            let to = raw_moves.moves[i + 1];
            if from == 28 { // White King
                assert!(to != 35 && to != 36 && to != 37, "King move to illegal adjacent king square {} was generated", to);
            }
        }
    }

    /// Finds a move by its algebraic notation, failing with the full move list when absent.
    fn find_move(moves: &[Turn], notation: &str) -> Turn {
        *moves
            .iter()
            .find(|m| m.to_algebraic() == notation)
            .unwrap_or_else(|| {
                let generated: Vec<String> = moves.iter().map(|m| m.to_algebraic()).collect();
                panic!("{} was not generated; got {:?}", notation, generated)
            })
    }

    fn has_move(moves: &[Turn], notation: &str) -> bool {
        moves.iter().any(|m| m.to_algebraic() == notation)
    }

    /// A king in check may not retreat along the ray of the slider checking it. The square behind
    /// the king only looks safe while the king is still standing in the way, which is why the
    /// attack test lifts it out of the occupancy.
    #[test]
    fn king_may_not_retreat_along_the_checking_ray_test() {
        let service = Service::new();
        // White king e2, black rook e8: the king is in check along the e-file.
        let mut board = service.fen.set_fen("4r3/8/8/8/8/8/4K3/8 w - - 0 1");
        let moves = generate_valid_moves_list(&mut board);

        assert!(!has_move(&moves, "e2e1"), "retreating to e1 stays on the rook's file");
        assert!(!has_move(&moves, "e2e3"), "advancing to e3 stays on the rook's file");
        assert!(has_move(&moves, "e2d2") && has_move(&moves, "e2f2"), "sideways escapes are legal");
        assert_eq!(moves.len(), 6, "only the six squares off the e-file are legal");
    }

    /// En passant can be the very capture that answers a check, in which case the destination
    /// square is not on the check mask at all — the checker stands next to it.
    #[test]
    fn en_passant_may_capture_the_checking_pawn_test() {
        let service = Service::new();
        // Black has just played d7-d5, checking the white king on e4 from d5.
        let mut board = service.fen.set_fen("4k3/8/8/3pP3/4K3/8/8/8 w - d6 0 1");
        let moves = generate_valid_moves_list(&mut board);

        assert!(has_move(&moves, "e5d6"), "exd6 e.p. removes the checking pawn and is legal");
    }

    /// The horizontal en passant pin: the capture clears two squares from the rank at once and
    /// exposes the own king to a rook that neither square alone would have exposed.
    #[test]
    fn en_passant_exposing_the_king_along_the_rank_test() {
        let service = Service::new();
        // White king a5, white pawn c5, black pawn d5, black rook h5, all on the fifth rank.
        let mut board = service.fen.set_fen("3k4/8/8/K1Pp3r/8/8/8/8 w - d6 0 1");
        let moves = generate_valid_moves_list(&mut board);

        assert!(!has_move(&moves, "c5d6"), "cxd6 e.p. would open the rank onto the king");
        assert!(has_move(&moves, "c5c6"), "the ordinary push stays legal");
    }

    /// A promoting pawn stands on the very ray its new piece would check along. The check squares
    /// are built before the pawn leaves, so the promotion has to be re-evaluated without it.
    #[test]
    fn promotion_checks_down_the_file_it_vacates_test() {
        let service = Service::new();
        // White pawn e7, black king e1: the pawn itself is the only thing on the e-file.
        let mut board = service.fen.set_fen("8/4P3/8/8/8/8/8/4k2K w - - 0 1");
        let moves = generate_valid_moves_list(&mut board);

        assert!(find_move(&moves, "e7e8q").gives_check, "the promoted queen checks down the e-file");
        assert!(find_move(&moves, "e7e8r").gives_check, "so does a rook");
        assert!(!find_move(&moves, "e7e8b").gives_check, "a bishop does not");
        assert!(!find_move(&moves, "e7e8n").gives_check, "and neither does a knight");
    }

    /// Castling delivers its check with the rook, not with the king.
    #[test]
    fn castling_rook_gives_check_test() {
        let service = Service::new();
        // White castles short; the rook lands on f1 with the black king on f8.
        let mut board = service.fen.set_fen("5k2/8/8/8/8/8/8/4K2R w K - 0 1");
        let moves = generate_valid_moves_list(&mut board);

        assert!(find_move(&moves, "e1g1").gives_check, "the rook on f1 checks along the f-file");
    }

    /// A king that stops blocking one of its own sliders delivers a discovered check.
    #[test]
    fn king_move_gives_discovered_check_test() {
        let service = Service::new();
        // White rook e1, white king e4, black king e8: the king is the only piece in the way.
        let mut board = service.fen.set_fen("4k3/8/8/8/4K3/8/8/4R3 w - - 0 1");
        let moves = generate_valid_moves_list(&mut board);

        assert!(find_move(&moves, "e4d4").gives_check, "stepping off the file uncovers the rook");
        assert!(find_move(&moves, "e4f3").gives_check, "so does any other square off the file");
        assert!(!find_move(&moves, "e4e5").gives_check, "staying on the file keeps it blocked");
        assert!(!find_move(&moves, "e4e3").gives_check, "in either direction");
    }

    #[test]
    fn test_streamlined_move_validation_gives_check() {
        let service = Service::new();
        // White Rook on e1, Black King on e8. Move e1->e7 gives check.
        let mut board = service.fen.set_fen("4k3/8/8/8/8/8/8/4R3 w - - 0 1");
        let moves = generate_valid_moves_list(&mut board);
        let gives_check_move = moves.iter().find(|m| m.from == 4 && m.to == 52); // e1e7
        assert!(gives_check_move.is_some(), "e1e7 move should be generated");
        assert!(gives_check_move.unwrap().gives_check, "e1e7 move should be marked as gives_check");
    }

    #[test]
    fn test_generate_moves_list_capture_only_filtering() {
        let service = Service::new();
        // Kiwipete position
        let board = service.fen.set_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        let mut raw_all_moves = crate::model::MoveRawList::new();
        let masks = service.move_gen.compute_node_masks(&board);
        service.move_gen.generate_moves_list_for_piece(&board, 0, false, &masks, &mut raw_all_moves);

        let mut raw_capture_moves = crate::model::MoveRawList::new();
        let masks = service.move_gen.compute_node_masks(&board);
        service.move_gen.generate_moves_list_for_piece(&board, 0, true, &masks, &mut raw_capture_moves);

        assert!(raw_capture_moves.len < raw_all_moves.len, "Captures-only raw move list should be smaller than all moves list");
        assert!(raw_capture_moves.len > 0, "Captures-only list should find captures in Kiwipete position");

        // Verify that every move in raw_capture_moves lands on an opponent piece
        let opp_pieces = board.black_pieces;
        for i in (0..raw_capture_moves.len).step_by(2) {
            let to = raw_capture_moves.moves[i + 1] as u8;
            assert_ne!(opp_pieces & (1u64 << to), 0, "Target square {} must be an opponent piece in captures-only mode", to);
        }
    }

    #[test]
    fn test_promotion_gives_check_independence() {
        let service = Service::new();

        // Position: White Pawn on a7, Black King on e8.
        // Queen/Rook promotions on a8 give check along the 8th rank.
        // Knight/Bishop promotions on a8 do NOT give check to e8.
        let mut board = service.fen.set_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1");
        let moves = generate_valid_moves_list(&mut board);

        let queen_promo = moves.iter().find(|m| m.from == 48 && m.to == 56 && m.promotion == 14);
        let knight_promo = moves.iter().find(|m| m.from == 48 && m.to == 56 && m.promotion == 12);
        let rook_promo = moves.iter().find(|m| m.from == 48 && m.to == 56 && m.promotion == 11);
        let bishop_promo = moves.iter().find(|m| m.from == 48 && m.to == 56 && m.promotion == 13);

        assert!(queen_promo.is_some(), "Queen promotion should exist");
        assert!(knight_promo.is_some(), "Knight promotion should exist");
        assert!(rook_promo.is_some(), "Rook promotion should exist");
        assert!(bishop_promo.is_some(), "Bishop promotion should exist");

        assert!(queen_promo.unwrap().gives_check, "Queen promotion on a8 must give check to e8");
        assert!(rook_promo.unwrap().gives_check, "Rook promotion on a8 must give check to e8");
        assert!(!knight_promo.unwrap().gives_check, "Knight promotion on a8 must NOT give check to e8");
        assert!(!bishop_promo.unwrap().gives_check, "Bishop promotion on a8 must NOT give check to e8");
    }
}
