
pub struct Stats {
    created_nodes: usize,
    calculated_nodes: usize,
    eval_nodes: usize,
    calc_time_ms: usize,
    zobrist_hit: usize,
}

impl Stats {

    pub fn new() -> Stats {
       Stats { calc_time_ms: 0, calculated_nodes: 0, created_nodes: 0, eval_nodes: 0, zobrist_hit: 0}
    }

    pub fn add_created_nodes(&mut self, value: usize) {
        self.created_nodes += value;
    }

    pub fn add_calculated_nodes(&mut self, value: usize) {
        self.calculated_nodes += value;
    }

    pub fn add_eval_nodes(&mut self, value: usize) {
        self.eval_nodes += value;
    }

    pub fn add_zobrist_hit(&mut self, value: usize) {
        self.zobrist_hit += value;
    }

    pub fn set_calc_time(&mut self, value: usize) {
        self.calc_time_ms = value;
    }

    pub fn reset_stats(&mut self) {
        self.created_nodes = 0;
        self.calculated_nodes = 0;
        self.eval_nodes = 0;
        self.calc_time_ms = 0;
        self.zobrist_hit = 0;
    }

    pub fn to_string(&self) -> String {
        format!("Cre_{}\tCal_{}\tEva_{}\tN/s_{}K CF_0.{}\tZb_0.{}",
        self.created_nodes,
        self.calculated_nodes,
        self.eval_nodes,
        self.created_nodes / (self.calc_time_ms + 1),
        100 - (self.calculated_nodes * 100 / if self.created_nodes == 0 { 1 } else { self.created_nodes }),
        self.zobrist_hit)
        //(self.zobrist_hit * 100 / self.eval_nodes))
    }

}