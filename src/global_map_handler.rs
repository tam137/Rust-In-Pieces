use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::Sender;
use std::collections::HashMap;

use crate::SegQueue;

use crate::zobrist;
use crate::{zobrist::ZobristTable, ThreadSafeDataMap};
use crate:: model::{Board, DataMap, DataMapKey, SearchResult, Turn};

use crate::model::RIP_COULDN_LOCK_GLOBAL_MAP;
use crate::model::RIP_COULDN_LOCK_MUTEX;
use crate::model::RIP_MISSED_DM_KEY;
use crate::model::RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE;


/// global map builder: Its not checked here BUT ONLY CALL CONSTRUCTOR ONCE!
/// Everything in here and the map itself is an Arc<Mutex>>
pub fn create_new_global_map() -> Arc<RwLock<DataMap>> {

    let global_map = Arc::new(RwLock::new(DataMap::new()));

    let logger: Arc<dyn Fn(String) + Send + Sync> = Arc::new(|_msg: String| {
        // empty logging function but can be applied by uci "debug on"
    });

    let debug_flag = false;
    let stop_flag = false;
    let zobrist_table = Arc::new(ZobristTable::new());
    let search_result_vector = Arc::new(Mutex::new(Vec::default()));
    let pv_nodes_map = Arc::new(Mutex::new(HashMap::new()));
    let pv_nodes_len = Arc::new(AtomicI32::new(0));

    {
        let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
        global_map_value.insert(DataMapKey::StopFlag, stop_flag);
        global_map_value.insert(DataMapKey::DebugFlag, debug_flag);
        global_map_value.insert(DataMapKey::Logger, logger);
        global_map_value.insert(DataMapKey::ZobristTable, zobrist_table);
        global_map_value.insert(DataMapKey::SearchResults, search_result_vector);
        global_map_value.insert(DataMapKey::PvNodes, pv_nodes_map);
        global_map_value.insert(DataMapKey::PvNodesLen, pv_nodes_len);
    }
    global_map.clone()
}


pub fn add_hash_sender(global_map: &ThreadSafeDataMap, producer_queue: Arc<SegQueue<(u64, i16)>>) {
    let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    global_map_value.insert(DataMapKey::HashSender, producer_queue);
}

pub fn add_std_in_sender(global_map: &ThreadSafeDataMap, sender: Sender<String>) {
    let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    global_map_value.insert(DataMapKey::StdInSender, sender.clone());
}

pub fn add_game_command_sender(global_map: &ThreadSafeDataMap, sender: Sender<String>) {
    let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    global_map_value.insert(DataMapKey::GameCommandSender, sender.clone());
}

pub fn add_log_buffer_sender(global_map: &ThreadSafeDataMap, sender: Sender<String>) {
    let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    global_map_value.insert(DataMapKey::LogBufferSender, sender.clone());
}

pub fn get_zobrist_table(global_map: &ThreadSafeDataMap) -> Arc<ZobristTable> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Arc<ZobristTable>>(DataMapKey::ZobristTable)
        .expect("RIP Can not find ZobristTable")
        .clone()
}

/// When debug flag is true, also a logger function should be applied
pub fn is_debug_flag(global_map: &ThreadSafeDataMap) -> bool {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<bool>(DataMapKey::DebugFlag)
        .expect("RIP Can not find debug flag")
        .clone()
}

pub fn is_stop_flag(global_map: &ThreadSafeDataMap) -> bool {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<bool>(DataMapKey::StopFlag)
        .expect("RIP Can not find stop flag")
        .clone()
}

pub fn _get_search_results(global_map: &ThreadSafeDataMap) -> Vec<SearchResult> {
    global_map.read().expect("RIP Ccould not lock global map")
        .get_data::<Arc<Mutex<Vec<SearchResult>>>>(DataMapKey::SearchResults)
        .expect("RIP can not find search results")
        .lock()
        .expect("RIP can not lock search results")
        .clone()
}

pub fn get_max_completed_result_depth(global_map: &ThreadSafeDataMap) -> i32 {
    let mut results = global_map.read().expect("RIP Could not lock global map")
        .get_data::<Arc<Mutex<Vec<SearchResult>>>>(DataMapKey::SearchResults)
        .expect("RIP can not find search results")
        .lock()
        .expect("RIP can not lock search results")
        .clone();

    results = results.iter().filter(|sr| sr.completed)
        .cloned()
        .collect();

    results.sort_unstable_by(|a, b| b.calculated_depth.cmp(&a.calculated_depth));
    if let Some(sr) = results.get(0) {
        sr.get_depth()
    }
    else {
        0
    }
}

pub fn push_search_result(global_map: &ThreadSafeDataMap, search_result: SearchResult) {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Arc<Mutex<Vec<SearchResult>>>>(DataMapKey::SearchResults)
        .expect("RIP Can not find search results")
        .lock()
        .expect("RIP Can not lock Search Results")
        .push(search_result);
}

pub fn clear_search_result(global_map: &ThreadSafeDataMap) {
    let global_map_value = global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    let search_results = global_map_value.get_data::<Arc<Mutex<Vec<SearchResult>>>>(DataMapKey::SearchResults)
        .expect(RIP_MISSED_DM_KEY);
    let mut search_result_guard = search_results.lock().expect(RIP_COULDN_LOCK_MUTEX);
    search_result_guard.clear();
    
}

pub fn set_stop_flag(global_map: &ThreadSafeDataMap, value: bool) {
    let mut global_map_value = global_map.write().expect("RIP Could not lock global map");
    global_map_value.insert(DataMapKey::StopFlag, value);
}

pub fn set_debug_flag(global_map: &ThreadSafeDataMap, value: bool) {
    let mut global_map_value = global_map.write().expect("RIP Could not lock global map");
    global_map_value.insert(DataMapKey::DebugFlag, value);
}

pub fn get_hash_sender(global_map: &ThreadSafeDataMap) -> Arc<SegQueue<(u64, i16)>> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Arc<SegQueue<(u64, i16)>>>(DataMapKey::HashSender)
        .expect("RIP Can not find hash sender").clone()
}

pub fn get_std_in_sender(global_map: &ThreadSafeDataMap) -> Sender<String> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Sender<String>>(DataMapKey::StdInSender)
        .expect("RIP Can not find std in sender")
        .clone()
}

pub fn get_game_command_sender(global_map: &ThreadSafeDataMap) -> Sender<String> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Sender<String>>(DataMapKey::GameCommandSender)
        .expect("RIP Can not find game command sender")
        .clone()
}

pub fn get_log_buffer_sender(global_map: &ThreadSafeDataMap) -> Sender<String> {
    global_map
        .read()
        .ok()
        .and_then(|map| map.get_data::<Sender<String>>(DataMapKey::LogBufferSender).cloned())
        .unwrap_or_else(|| {
            let (tx, _rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                // Dummy-Receiver
                for _ in _rx {}
            });
            tx
        })
}

pub fn get_pv_node_for_hash(global_map: &ThreadSafeDataMap, hash: u64) -> Option<Turn> {
    let global_map_read = global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    
    let arc_mutex = global_map_read
        .get_data::<Arc<Mutex<HashMap<u64, Turn>>>>(DataMapKey::PvNodes)
        .expect(RIP_MISSED_DM_KEY);

    let hash_map = arc_mutex.lock().expect(RIP_COULDN_LOCK_MUTEX);
    hash_map.get(&hash).cloned()
}

/// returns the calculated depth from the PV Thread stored in DataMapKey::PvNodesLen
pub fn get_pv_nodes_calculated_depth(global_map: &ThreadSafeDataMap) -> usize {
    let global_map_value = global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    let pv_len = global_map_value
        .get_data::<Arc<AtomicI32>>(DataMapKey::PvNodesLen)
        .expect(RIP_MISSED_DM_KEY);
    
    pv_len.load(std::sync::atomic::Ordering::SeqCst) as usize    
}

/// returns the len of the stored pv node move row
pub fn _get_pv_nodes_len(global_map: &ThreadSafeDataMap) -> usize {
    let global_map_value = global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    let pv_moves_row = global_map_value
        .get_data::<Arc<Mutex<HashMap<u64, Turn>>>>(DataMapKey::PvNodes)
        .expect(RIP_MISSED_DM_KEY);
    
    let pv_moves_len = pv_moves_row.lock().expect(RIP_COULDN_LOCK_MUTEX).len();
    pv_moves_len
}

/// Cleares two keys: DataMapKey::PvNodes and DataMapKey::PvNodesLen
pub fn clear_pv_nodes(global_map: &ThreadSafeDataMap) {
    let global_map_value = global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    let arc_mutex = global_map_value
        .get_data::<Arc<Mutex<HashMap<u64, Turn>>>>(DataMapKey::PvNodes)
        .expect(RIP_MISSED_DM_KEY);

    let mut hash_map = arc_mutex
        .lock()
        .expect(RIP_COULDN_LOCK_MUTEX);

    hash_map.clear();

    let pv_len = global_map_value
        .get_data::<Arc<AtomicI32>>(DataMapKey::PvNodesLen)
        .expect(RIP_MISSED_DM_KEY);
    pv_len.store(0, std::sync::atomic::Ordering::SeqCst);
}

/// Generates a pv_nodes Map<u64, Turn> and stores it to the global_map
/// The board is mutable but not changed by the end of calculation
/// PV Nodes are only set if move row is longer then previous stored pv
/// PV nodes len is not stored here. Call set_pv_nodes_len()
pub fn set_pv_nodes(global_map: &ThreadSafeDataMap, move_row: &Vec<Turn>, board: &mut Board) {

    let logger = get_log_buffer_sender(global_map);

    let pv_node_len = get_pv_nodes_calculated_depth(global_map);
    let move_row_len = move_row.len();

    if pv_node_len >= move_row_len {
        logger.send(format!("Stored Pv nodes len ({}) >= new move row ({}), no save", pv_node_len, move_row_len))
            .expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
        return;
    }

    let mut new_pv_node_map = HashMap::new();
    let old_board = board.clone();
    
    for turn in move_row {
        let hash = zobrist::gen(board);
        new_pv_node_map.insert(hash, turn.clone());
        board.do_move(turn);
    }
    let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    let new_pv_node_map_arc = Arc::new(Mutex::new(new_pv_node_map));
    global_map_value.insert(DataMapKey::PvNodes, new_pv_node_map_arc);

    logger.send(format!("Save new pv len ({})", move_row_len)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
    *board = old_board;
}

/// Sets the calculated depth of stored PV nodes in DataMapKey::PvNodesLen
pub fn set_pv_nodes_len(global_map: &ThreadSafeDataMap, pv_calculated_depth: i32) {
    let global_map_value = global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    let pv_nodes_len = global_map_value.get_data::<Arc<AtomicI32>>(DataMapKey::PvNodesLen)
        .expect(RIP_MISSED_DM_KEY);
    pv_nodes_len.store(pv_calculated_depth, Ordering::SeqCst);
}

pub fn _is_calculated_at_least_one_finished_search_result(global_map: &ThreadSafeDataMap) -> bool {
    let global_map_value = global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    let search_results = global_map_value
        .get_data::<Arc<Mutex<Vec<SearchResult>>>>(DataMapKey::SearchResults)
        .expect(RIP_MISSED_DM_KEY)
        .lock()
        .expect(RIP_COULDN_LOCK_MUTEX);

    search_results.iter().any(|sr| sr.completed)
}



#[cfg(test)]
mod tests {

    use crate::{global_map_handler::{self, *}, service::Service};

    #[test]
    fn create_new_global_map_test() {
        let global_map = create_new_global_map();    
        let debug_flag = is_debug_flag(&global_map);    
        assert!(debug_flag == false);
    }


    #[test]
    fn pv_nodes_generation_test() {
        let global_map = create_new_global_map();
        let service = Service::new();

        let mut board = service.fen.set_init_board();
        let hash = zobrist::gen(&board);
        let turn = Turn::_new_to_from(85, 65);

        let mut board_2 = board.clone();
        board_2.do_move(&turn);
        let hash_2 = zobrist::gen(&board_2);
        let turn_2 = Turn::_new_to_from(35, 55);

        let mut move_row = Vec::default();
        move_row.push(turn);
        move_row.push(turn_2);

        set_pv_nodes(&global_map, &move_row, &mut board);

        assert_eq!(85, get_pv_node_for_hash(&global_map, hash).unwrap().from);
        assert_eq!(65, get_pv_node_for_hash(&global_map, hash).unwrap().to);

        assert_eq!(35, get_pv_node_for_hash(&global_map, hash_2).unwrap().from);
        assert_eq!(55, get_pv_node_for_hash(&global_map, hash_2).unwrap().to);

        let unused_hash = 123 as u64;

        let none_value = get_pv_node_for_hash(&global_map, unused_hash);
        assert_eq!(none_value, None);

        assert_eq!(2, _get_pv_nodes_len(&global_map));

        global_map_handler::clear_pv_nodes(&global_map);
        assert_eq!(0, _get_pv_nodes_len(&global_map));

        let mut move_row = Vec::default();
        let turn = Turn::_new_to_from(85, 65);
        move_row.push(turn);
        set_pv_nodes(&global_map, &move_row, &mut board);
        assert_eq!(1, _get_pv_nodes_len(&global_map));

        let mut move_row = Vec::default();
        let turn = Turn::_new_to_from(85, 65);
        let turn_2 = Turn::_new_to_from(35, 55);
        move_row.push(turn);
        move_row.push(turn_2);
        set_pv_nodes(&global_map, &move_row, &mut board);
        assert_eq!(2, _get_pv_nodes_len(&global_map));

        let mut move_row = Vec::default();
        let turn = Turn::_new_to_from(85, 65);
        move_row.push(turn);
        set_pv_nodes(&global_map, &move_row, &mut board);
        assert_eq!(1, _get_pv_nodes_len(&global_map));

    }
}
