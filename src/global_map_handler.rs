use std::sync::{Arc, Mutex, RwLock};

use crate::{zobrist::ZobristTable, ThreadSafeDataMap};
use crate:: model::{DataMap, DataMapKey, LoggerFnType};


/// global map builder: Its not checked here BUT ONLY CALL IT ONCE!
/// Everything in here and the map itself is an Arc<Mutex>>
pub fn create_new_global_map() {

    let global_map = Arc::new(RwLock::new(DataMap::new()));

    let logger: Arc<dyn Fn(String) + Send + Sync> = Arc::new(|_msg: String| {
        // empty logging function but can be applied by uci "debug on"
    });

    let debug_flag = Arc::new(Mutex::new(false));
    let stop_flag = Arc::new(Mutex::new(false));
    let zobrist_table = Arc::new(Mutex::new(ZobristTable::new()));

    {
        let mut global_map_value = global_map.write().expect("RIP Could not lock global map");
        global_map_value.insert(DataMapKey::StopFlag, stop_flag.clone());
        global_map_value.insert(DataMapKey::DebugFlag, debug_flag.clone());
        global_map_value.insert(DataMapKey::Logger, logger.clone());
        global_map_value.insert(DataMapKey::ZobristTable, zobrist_table.clone());
    }
}


pub fn get_zobrist_table(global_map: &ThreadSafeDataMap) -> Arc<Mutex<ZobristTable>> {
    global_map.read().expect("RIP Can not lock global map")
        .get_data::<Arc<Mutex<ZobristTable>>>(DataMapKey::ZobristTable)
        .expect("RIP Can not find ZobristTable")
        .clone()
}

pub fn get_logger(global_map: &ThreadSafeDataMap) -> LoggerFnType {
    global_map.read().expect("RIP Can not lock global map")
        .get_data::<LoggerFnType>(DataMapKey::Logger)
        .expect("RIP Can not find logger")
        .clone()
}

pub fn get_debug_flag(global_map: &ThreadSafeDataMap) -> Arc<Mutex<bool>> {
    global_map.read().expect("RIP Can not lock global map")
        .get_data::<Arc<Mutex<bool>>>(DataMapKey::DebugFlag)
        .expect("RIP Can not find debug flag")
        .clone()
}

pub fn get_stop_flag(global_map: &ThreadSafeDataMap) -> Arc<Mutex<bool>> {
    global_map.read().expect("RIP Can not lock global map")
        .get_data::<Arc<Mutex<bool>>>(DataMapKey::StopFlag)
        .expect("RIP Can not find stop flag")
        .clone()
}