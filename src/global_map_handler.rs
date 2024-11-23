use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::Sender;

use crate::{zobrist::ZobristTable, ThreadSafeDataMap};
use crate:: model::{DataMap, DataMapKey, LoggerFnType};

use crate::model::RIP_COULDN_LOCK_GLOBAL_MAP;


/// global map builder: Its not checked here BUT ONLY CALL IT ONCE!
/// Everything in here and the map itself is an Arc<Mutex>>
pub fn create_new_global_map() -> Arc<RwLock<DataMap>> {

    let global_map = Arc::new(RwLock::new(DataMap::new()));

    let logger: Arc<dyn Fn(String) + Send + Sync> = Arc::new(|_msg: String| {
        // empty logging function but can be applied by uci "debug on"
    });

    let debug_flag = Arc::new(Mutex::new(false));
    let stop_flag = Arc::new(Mutex::new(false));
    let zobrist_table = Arc::new(RwLock::new(ZobristTable::new()));

    {
        let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
        global_map_value.insert(DataMapKey::StopFlag, stop_flag.clone());
        global_map_value.insert(DataMapKey::DebugFlag, debug_flag.clone());
        global_map_value.insert(DataMapKey::Logger, logger.clone());
        global_map_value.insert(DataMapKey::ZobristTable, zobrist_table.clone());
    }
    global_map.clone()
}

/// add the hash sender. Not added in tests
pub fn add_hash_sender(global_map: &ThreadSafeDataMap, sender: Sender<(u64, i16)>) {
    let mut global_map_value = global_map.write().expect(RIP_COULDN_LOCK_GLOBAL_MAP);
    global_map_value.insert(DataMapKey::HashSender, sender.clone());
}

pub fn get_zobrist_table(global_map: &ThreadSafeDataMap) -> Arc<RwLock<ZobristTable>> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Arc<RwLock<ZobristTable>>>(DataMapKey::ZobristTable)
        .expect("RIP Can not find ZobristTable")
        .clone()
}

pub fn get_logger(global_map: &ThreadSafeDataMap) -> LoggerFnType {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<LoggerFnType>(DataMapKey::Logger)
        .expect("RIP Can not find logger")
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

pub fn get_hash_sender(global_map: &ThreadSafeDataMap) -> Sender<(u64, i16)> {
    global_map.read().expect(RIP_COULDN_LOCK_GLOBAL_MAP)
        .get_data::<Sender<(u64, i16)>>(DataMapKey::HashSender)
        .expect("RIP Can not find hash sender")
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
