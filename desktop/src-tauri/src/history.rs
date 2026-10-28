use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub word: String,
    /// Unix 毫秒时间戳
    pub at: u64,
    /// 触发来源：popup / hotkey / manual
    pub source: String,
    /// 命中的首个词典名（本地词库名或在线引擎名）
    #[serde(default)]
    pub dict: String,
}

fn path_of(dir: &Path) -> PathBuf {
    dir.join("history.json")
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn load(dir: &Path) -> Vec<HistoryItem> {
    let raw = match fs::read_to_string(path_of(dir)) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str::<Vec<HistoryItem>>(&raw).unwrap_or_default()
}

pub fn save(dir: &Path, items: &[HistoryItem]) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = path_of(dir);
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_vec(items).map_err(|e| e.to_string())?;
    {
        let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        file.write_all(&body).map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// 追加一条历史：相邻重复合并，超出上限丢弃最旧记录
pub fn push(items: &mut Vec<HistoryItem>, mut item: HistoryItem, limit: usize) {
    if let Some(last) = items.first() {
        if last.word.eq_ignore_ascii_case(&item.word) {
            items.remove(0);
        }
    }
    if item.at == 0 {
        item.at = now_ms();
    }
    items.insert(0, item);
    if items.len() > limit.max(50) {
        items.truncate(limit.max(50));
    }
}
