use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::Sender;

use crate::{zobrist::ZobristTable, ThreadSafeDataMap};
use crate:: model::{DataMap, DataMapKey, SearchResult};

use crate::model::RIP_COULDN_LOCK_GLOBAL_MAP;


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

    {
        let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
        global_map_value.insert(DataMapKey::StopFlag, stop_flag);
        global_map_value.insert(DataMapKey::DebugFlag, debug_flag);
        global_map_value.insert(DataMapKey::Logger, logger);
        global_map_value.insert(DataMapKey::ZobristTable, zobrist_table);
        global_map_value.insert(DataMapKey::SearchResults, search_result_vector);
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
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Sender<String>>(DataMapKey::LogBufferSender)
        .expect("RIP Can not find log msg sender")
        .clone()
}



#[cfg(test)]
mod tests {

    use crate::{global_map_handler::*, model::RIP_COULDN_LOCK_ZOBRIST};

    #[test]
    fn create_new_global_map_test() {
        let global_map = create_new_global_map();
    
        let debug_flag_mutex = get_debug_flag(&global_map);
        let debug_flag = debug_flag_mutex.lock().expect(RIP_COULDN_LOCK_ZOBRIST);
    
        assert!(*debug_flag == false);
    }
}
