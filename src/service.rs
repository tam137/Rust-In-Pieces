use crate::eval_service::EvalService;
use crate::fen_service::FenService;
use crate::move_gen_service::MoveGenService;
use crate::search_service::SearchService;
use crate::zobrist::ZobristTable;
use crate::stdout_wrapper::StdoutWrapper;

pub struct Service {
    pub fen: FenService,
    pub move_gen: MoveGenService,
    pub search: SearchService,
    pub eval: EvalService,
    pub zobrist: ZobristTable,
    pub stdout: StdoutWrapper,
}

impl Service {
    pub fn new() -> Self {
        Service {
            fen: FenService,
            move_gen: MoveGenService::new(),
            search: SearchService::new(),
            eval: EvalService::new(),
            zobrist: ZobristTable::new(),
            stdout: StdoutWrapper,
        }
    }
}
