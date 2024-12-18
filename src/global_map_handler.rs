use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::Sender;
use std::collections::HashMap;

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

    let debug_flag = Arc::new(Mutex::new(false));
    let stop_flag = Arc::new(Mutex::new(false));
    let zobrist_table = Arc::new(RwLock::new(ZobristTable::new()));
    let search_result_vector = Arc::new(Mutex::new(Vec::default()));
    let pv_nodes_map = Arc::new(Mutex::new(HashMap::new()));

    {
        let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
        global_map_value.insert(DataMapKey::StopFlag, stop_flag);
        global_map_value.insert(DataMapKey::DebugFlag, debug_flag);
        global_map_value.insert(DataMapKey::Logger, logger);
        global_map_value.insert(DataMapKey::ZobristTable, zobrist_table);
        global_map_value.insert(DataMapKey::SearchResults, search_result_vector);
        global_map_value.insert(DataMapKey::PvNodes, pv_nodes_map);
    }
    global_map.clone()
}


pub fn add_hash_sender(global_map: &ThreadSafeDataMap, sender: Sender<(u64, i16)>) {
    let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    global_map_value.insert(DataMapKey::HashSender, sender.clone());
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

pub fn get_zobrist_table(global_map: &ThreadSafeDataMap) -> Arc<RwLock<ZobristTable>> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Arc<RwLock<ZobristTable>>>(DataMapKey::ZobristTable)
        .expect("RIP Can not find ZobristTable")
        .clone()
}

pub fn get_debug_flag(global_map: &ThreadSafeDataMap) -> Arc<Mutex<bool>> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Arc<Mutex<bool>>>(DataMapKey::DebugFlag)
        .expect("RIP Can not find debug flag")
        .clone()
}

/// When debug flag is true, also a logger function should be applied
pub fn is_debug_flag(global_map: &ThreadSafeDataMap) -> bool {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Arc<Mutex<bool>>>(DataMapKey::DebugFlag)
        .expect("RIP Can not find debug flag")
        .lock()
        .expect("RIP Can not lock debug flag")
        .clone()
}

pub fn get_stop_flag(global_map: &ThreadSafeDataMap) -> Arc<Mutex<bool>> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Arc<Mutex<bool>>>(DataMapKey::StopFlag)
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

pub fn push_search_result(global_map: &ThreadSafeDataMap, search_result: SearchResult) {
    global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Arc<Mutex<Vec<SearchResult>>>>(DataMapKey::SearchResults)
        .expect("RIP Can not find search results")
        .lock()
        .expect("RIP Can not lock Search Results")
        .push(search_result);
}

pub fn _clear_search_result(global_map: &ThreadSafeDataMap) {
    let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    global_map_value.insert(DataMapKey::SearchResults, Arc::new(Mutex::new(Vec::default())));
}

pub fn is_stop_flag(global_map: &ThreadSafeDataMap) -> bool {
    let global_map_value = global_map.read().expect("RIP Could not lock global map");
    if let Some(flag) = global_map_value.get_data::<Arc<Mutex<bool>>>(DataMapKey::StopFlag) {
        let stop_flag = flag.lock().expect("RIP Can not read stop_flag");
        *stop_flag
    } else {
        panic!("RIP Cant read stop flag");
    }
}

pub fn set_stop_flag(global_map: &ThreadSafeDataMap, value: bool) {
    let global_map_value = global_map.write().expect("RIP Could not lock global map");
    if let Some(flag) = global_map_value.get_data::<Arc<Mutex<bool>>>(DataMapKey::StopFlag) {
        let mut stop_flag = flag.lock().expect("RIP Can not read stop_flag");
        *stop_flag = value;
    } else {
        panic!("RIP Cant read stop flag");
    }
}

pub fn get_hash_sender(global_map: &ThreadSafeDataMap) -> Sender<(u64, i16)> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Sender<(u64, i16)>>(DataMapKey::HashSender)
        .expect("RIP Can not find hash sender")
        .clone()
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

pub fn get_pv_node_for_hash(global_map: &ThreadSafeDataMap, hash: u64) -> Result<Turn, String> {
    global_map
        .read()
        .map_err(|_| RIP_COULDN_LOCK_GLOBAL_MAP)?
        .get_data::<Arc<Mutex<HashMap<u64, Turn>>>>(DataMapKey::PvNodes)
        .ok_or_else(|| RIP_MISSED_DM_KEY)?
        .lock()
        .map_err(|_| RIP_COULDN_LOCK_MUTEX.to_string())
        .and_then(|map| {
            map
                .get(&hash)
                .cloned()
                .ok_or_else(|| "No matching key found in pv nodes map".to_string())
        })
}

pub fn get_pv_nodes_len(global_map: &ThreadSafeDataMap) -> usize {
    let global_map_value = global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    let arc_mutex = global_map_value
        .get_data::<Arc<Mutex<HashMap<u64, Turn>>>>(DataMapKey::PvNodes)
        .expect(RIP_MISSED_DM_KEY);
    
    let hash_map = arc_mutex
        .lock()
        .expect(RIP_COULDN_LOCK_MUTEX);

    hash_map.len()
}

pub fn clear_pv_nodes(global_map: &ThreadSafeDataMap) {
    let global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    let arc_mutex = global_map_value
        .get_data::<Arc<Mutex<HashMap<u64, Turn>>>>(DataMapKey::PvNodes)
        .expect(RIP_MISSED_DM_KEY);
    
    let mut hash_map = arc_mutex
        .lock()
        .expect(RIP_COULDN_LOCK_MUTEX);

    hash_map.clear();
}



/// Generates a pv_nodes Map<u64, Turn> and stores it to the global_map
/// The board is mutable but not changed by the end of calculation
/// PV Nodes are only set if move row is longer then previous stored pv
pub fn set_pv_nodes(global_map: &ThreadSafeDataMap, move_row: &Vec<Turn>, board: &mut Board) {

    let logger = get_log_buffer_sender(global_map);

    let pv_node_len = get_pv_nodes_len(global_map);
    let move_row_len = move_row.len();

    if pv_node_len >= move_row_len {
        logger.send(format!("Stored Pv Nodes Len ({}) > new move row ({})", pv_node_len, move_row_len))
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
    logger.send(format!("Stored new pv len ({})", move_row_len)).expect(RIP_COULDN_SEND_TO_LOG_BUFFER_QUEUE);
    *board = old_board;
}



#[cfg(test)]
mod tests {

    use crate::{global_map_handler::{self, *}, model::RIP_COULDN_LOCK_MUTEX, service::Service};

    #[test]
    fn create_new_global_map_test() {
        let global_map = create_new_global_map();
    
        let debug_flag_mutex = get_debug_flag(&global_map);
        let debug_flag = debug_flag_mutex.lock().expect(RIP_COULDN_LOCK_MUTEX);
    
        assert!(*debug_flag == false);
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

        let error_type = get_pv_node_for_hash(&global_map, unused_hash);
        assert!(matches!(error_type, Err(ref e) if e.is_ascii()));

        assert_eq!(2, get_pv_nodes_len(&global_map));

        global_map_handler::clear_pv_nodes(&global_map);
        assert_eq!(0, get_pv_nodes_len(&global_map));

        let mut move_row = Vec::default();
        let turn = Turn::_new_to_from(85, 65);
        move_row.push(turn);
        set_pv_nodes(&global_map, &move_row, &mut board);
        assert_eq!(1, get_pv_nodes_len(&global_map));

        let mut move_row = Vec::default();
        let turn = Turn::_new_to_from(85, 65);
        let turn_2 = Turn::_new_to_from(35, 55);
        move_row.push(turn);
        move_row.push(turn_2);
        set_pv_nodes(&global_map, &move_row, &mut board);
        assert_eq!(2, get_pv_nodes_len(&global_map));

        let mut move_row = Vec::default();
        let turn = Turn::_new_to_from(85, 65);
        move_row.push(turn);
        set_pv_nodes(&global_map, &move_row, &mut board);
        assert_eq!(2, get_pv_nodes_len(&global_map));

    }
}
