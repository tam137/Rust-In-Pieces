use crate::config::Config;
use crate::eval_service::EvalService;
use crate::fen_service::FenService;
use crate::move_gen_service::MoveGenService;
use crate::search_service::SearchService;
use crate::uci_parser_service::UciParserService;
use crate::stdout_wrapper::StdoutWrapper;
use crate::pawn_hash::PawnHashTable;

pub struct Service {
    pub fen: FenService,
    pub move_gen: MoveGenService,
    pub search: SearchService,
    pub eval: EvalService,
    pub stdout: StdoutWrapper,
    pub uci_parser: UciParserService,
    pub pawn_table: PawnHashTable,
}

impl Service {
    pub fn new() -> Self {
        Service {
            fen: FenService,
            move_gen: MoveGenService::new(),
            search: SearchService::new(),
            eval: EvalService::new(&Config::new()),
            stdout: StdoutWrapper,
            uci_parser: UciParserService::new(),
            pawn_table: PawnHashTable::new(Config::new().max_pawn_hash_entries),
        }
    }
}
