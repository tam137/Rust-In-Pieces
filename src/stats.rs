


pub struct Stats {
    created_nodes: usize,
    calculated_nodes: usize,
    eval_nodes: usize,
    calc_time_ms: usize,
}

impl Stats {

    pub fn new() -> Stats {
       Stats { calc_time_ms: 0, calculated_nodes: 0, created_nodes: 0, eval_nodes: 0}
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

    pub fn set_calc_time(&mut self, value: usize) {
        self.calc_time_ms = value;
    }

    pub fn reset_stats(&mut self) {
        self.created_nodes = 0;
        self.calculated_nodes = 0;
        self.eval_nodes = 0;
        self.calc_time_ms = 0;
    }

    pub fn to_string(&self) -> String {
        format!("Cre_{} Cal_{} Eva_{} N/s_{}K CF_0.{}",
        self.created_nodes,
        self.calculated_nodes,
        self.eval_nodes,
        self.created_nodes / (self.calc_time_ms + 1),
        100 - (self.calculated_nodes * 100 / if self.created_nodes == 0 { 1 } else { self.created_nodes }))
    }

}