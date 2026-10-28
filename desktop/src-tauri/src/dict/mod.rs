pub mod mdx;
pub mod sanitize;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;

use crate::config::DictItem;
use mdx::Dict;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DictHit {
    pub id: String,
    pub name: String,
    pub key: String,
    pub html: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DictError {
    pub id: String,
    pub name: String,
    pub message: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DictLookup {
    pub hits: Vec<DictHit>,
    pub errors: Vec<DictError>,
    pub suggestions: Vec<String>,
}

/// 单本词库的运行态：懒加载 + 记住最近一次错误
pub struct Runtime {
    pub id: String,
    pub path: PathBuf,
    pub name: String,
    pub enabled: AtomicBool,
    pub entry_count: AtomicU64,
    pub size: u64,
    pub mdd: Option<String>,
    loaded: Mutex<Option<Arc<Dict>>>,
    error: Mutex<Option<String>>,
}

impl Runtime {
    fn new(item: &DictItem) -> Self {
        Self {
            id: item.id.clone(),
            path: PathBuf::from(&item.path),
            name: item.name.clone(),
            enabled: AtomicBool::new(item.enabled),
            entry_count: AtomicU64::new(item.entry_count),
            size: item.size,
            mdd: item.mdd.clone(),
            loaded: Mutex::new(None),
            error: Mutex::new(None),
        }
    }

    /// 首次查询时才真正打开文件（大词典要读 key 块，放到阻塞线程做）
    pub fn ensure(&self) -> Result<Arc<Dict>, String> {
        if let Some(dict) = self.loaded.lock().ok().and_then(|g| g.clone()) {
            return Ok(dict);
        }
        let dict = Dict::open(self.id.clone(), &self.path, Some(self.name.clone()))?;
        self.entry_count.store(dict.entry_count, Ordering::Relaxed);
        let arc = Arc::new(dict);
        if let Ok(mut guard) = self.loaded.lock() {
            *guard = Some(arc.clone());
        }
        if let Ok(mut err) = self.error.lock() {
            *err = None;
        }
        Ok(arc)
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    pub fn last_error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|g| g.clone())
    }

    fn record_error(&self, message: String) {
        if let Ok(mut err) = self.error.lock() {
            *err = Some(message);
        }
    }

    pub fn invalidate(&self) {
        if let Ok(mut guard) = self.loaded.lock() {
            *guard = None;
        }
    }
}

#[derive(Default)]
pub struct DictStore {
    items: RwLock<Vec<Arc<Runtime>>>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DictInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub enabled: bool,
    pub loaded: bool,
    pub entry_count: u64,
    pub size: u64,
    pub mdd: Option<String>,
    pub error: Option<String>,
}

impl DictStore {
    /// 与配置对齐：路径不变的词库保留已加载状态，避免重复打开文件
    pub fn sync(&self, items: &[DictItem]) {
        let mut guard = match self.items.write() {
            Ok(g) => g,
            Err(_) => return,
        };
        let mut next: Vec<Arc<Runtime>> = Vec::with_capacity(items.len());
        for item in items {
            let reused = guard
                .iter()
                .find(|rt| rt.id == item.id && rt.path.as_path() == std::path::Path::new(&item.path))
                .cloned();
            match reused {
                Some(rt) => {
                    rt.enabled.store(item.enabled, Ordering::Relaxed);
                    next.push(rt);
                }
                None => next.push(Arc::new(Runtime::new(item))),
            }
        }
        *guard = next;
    }

    pub fn list(&self) -> Vec<DictInfo> {
        let guard = match self.items.read() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };
        guard
            .iter()
            .map(|rt| DictInfo {
                id: rt.id.clone(),
                name: rt.name.clone(),
                path: rt.path.to_string_lossy().to_string(),
                enabled: rt.enabled.load(Ordering::Relaxed),
                loaded: rt.is_loaded(),
                entry_count: rt.entry_count.load(Ordering::Relaxed),
                size: rt.size,
                mdd: rt.mdd.clone(),
                error: rt.last_error(),
            })
            .collect()
    }

    fn snapshot(&self) -> Vec<Arc<Runtime>> {
        self.items
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|rt| rt.enabled.load(Ordering::Relaxed))
            .collect()
    }

    pub fn runtime(&self, id: &str) -> Option<Arc<Runtime>> {
        self.items
            .read()
            .ok()?
            .iter()
            .find(|rt| rt.id == id)
            .cloned()
    }

    /// 阻塞查询：按配置顺序逐本查，保证结果面板顺序稳定
    pub fn lookup(&self, word: &str) -> DictLookup {
        let mut outcome = DictLookup::default();
        for rt in self.snapshot() {
            let dict = match rt.ensure() {
                Ok(dict) => dict,
                Err(err) => {
                    rt.record_error(err.clone());
                    outcome.errors.push(DictError {
                        id: rt.id.clone(),
                        name: rt.name.clone(),
                        message: err,
                    });
                    continue;
                }
            };
            match dict.lookup(word) {
                Ok(Some(hit)) => outcome.hits.push(DictHit {
                    id: rt.id.clone(),
                    name: rt.name.clone(),
                    key: hit.key,
                    html: hit.html,
                }),
                Ok(None) => {}
                Err(err) => {
                    rt.record_error(err.clone());
                    outcome.errors.push(DictError {
                        id: rt.id.clone(),
                        name: rt.name.clone(),
                        message: err,
                    });
                }
            }
        }
        if outcome.hits.is_empty() {
            outcome.suggestions = self.suggest(word, 8);
        }
        outcome
    }

    pub fn suggest(&self, word: &str, limit: usize) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for rt in self.snapshot() {
            if out.len() >= limit {
                break;
            }
            if let Ok(dict) = rt.ensure() {
                for key in dict.suggest(word, limit) {
                    if !out.iter().any(|k| k.eq_ignore_ascii_case(&key)) {
                        out.push(key);
                    }
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
        out
    }

    pub fn resource(&self, dict_id: &str, path: &str) -> Option<Vec<u8>> {
        let rt = self.runtime(dict_id)?;
        let dict = rt.ensure().ok()?;
        dict.resource(path)
    }
}

/// 生成稳定的词库 id（同一路径永远得到同一 id）
pub fn dict_id(path: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.to_lowercase().as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("d{hash:016x}")
}
