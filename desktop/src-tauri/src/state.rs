use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{self, AppConfig, DictItem};
use crate::dict::{dict_id, mdx::Dict, DictInfo, DictLookup, DictStore};
use crate::history::{self, HistoryItem};
use crate::net::{self, FetchResult};
use crate::win::{autostart, capture, hotkey, popup};

pub struct AppState {
    dir: OnceLock<PathBuf>,
    pub config: Mutex<AppConfig>,
    pub dicts: Arc<DictStore>,
    pub history: Mutex<Vec<HistoryItem>>,
    pub pinned: AtomicBool,
    /// 浮窗最近一次请求：窗口是懒创建的，事件可能在 WebView 挂载前就发出去了，
    /// 因此这里留一份快照供前端启动后主动取回。
    pub popup_seq: std::sync::atomic::AtomicU64,
    pub pending_popup: Mutex<Option<PopupRequest>>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PopupRequest {
    pub word: String,
    pub source: String,
    pub pinned: bool,
    pub seq: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LookupResponse {
    pub word: String,
    pub elapsed_ms: u64,
    pub local: DictLookup,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub version: String,
    pub config: AppConfig,
    pub dicts: Vec<DictInfo>,
    pub history: Vec<HistoryItem>,
    pub paused: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            dir: OnceLock::new(),
            config: Mutex::new(AppConfig::default()),
            dicts: Arc::new(DictStore::default()),
            history: Mutex::new(Vec::new()),
            pinned: AtomicBool::new(false),
            popup_seq: std::sync::atomic::AtomicU64::new(0),
            pending_popup: Mutex::new(None),
        }
    }

    pub fn dir(&self) -> PathBuf {
        self.dir.get().cloned().unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn config_snapshot(&self) -> AppConfig {
        self.config
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn init(&self, app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let dir = app.path().app_config_dir()?;
        std::fs::create_dir_all(&dir)?;
        let cfg = config::load(&dir);
        self.dicts.sync(&cfg.dicts);
        net::set_proxy(&cfg.online.proxy);
        let hist = history::load(&dir);
        if let Ok(mut guard) = self.config.lock() {
            *guard = cfg;
        }
        if let Ok(mut guard) = self.history.lock() {
            *guard = hist;
        }
        let _ = self.dir.set(dir);
        Ok(())
    }

    pub fn persist(&self, dir: &std::path::Path) -> Result<(), String> {
        config::save(dir, &self.config_snapshot())
    }

    pub fn set_config(&self, cfg: AppConfig) -> AppConfig {
        if let Ok(mut guard) = self.config.lock() {
            *guard = cfg.clone();
        }
        self.dicts.sync(&cfg.dicts);
        cfg
    }

    fn push_history(&self, word: &str, source: &str, dict_name: &str) {
        let limit = self.config_snapshot().history_limit;
        if let Ok(mut guard) = self.history.lock() {
            history::push(
                &mut guard,
                HistoryItem {
                    word: word.to_string(),
                    at: history::now_ms(),
                    source: source.to_string(),
                    dict: dict_name.to_string(),
                },
                limit,
            );
        }
        let _ = history::save(&self.dir(), &self.history_snapshot());
    }

    fn history_snapshot(&self) -> Vec<HistoryItem> {
        self.history
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// 取词后弹出浮窗（供鼠标钩子 / 快捷键调用）
pub fn capture_and_show(app: &AppHandle, source: &str) {
    if crate::win::hook::is_paused() {
        return;
    }
    let state = app.state::<AppState>();
    let cfg = state.config_snapshot();
    let Some(text) = capture::selected_text() else {
        log::info!("capture({source}): 未取到选中文本");
        return;
    };
    let word = clean_word(&text);
    let len = word.chars().count();
    let min = cfg.selection.min_chars.max(1);
    let max = cfg.selection.max_chars.max(min);
    if len < min || len > max {
        log::info!("capture({source}): 长度 {len} 超出 {min}..{max}，跳过");
        return;
    }
    log::info!("capture({source}): 命中 {len} 字符");
    popup::show_for(app, &word, source, cfg.selection.pin_on_open);
}

/// 去掉首尾空白与引号，把连续空白压成单空格
pub fn clean_word(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '“' | '”' | '‘' | '’' | '「' | '」'))
        .trim();
    let mut out = String::with_capacity(trimmed.len());
    let mut last_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(ch);
            last_space = false;
        }
    }
    out
}

#[tauri::command]
pub fn cmd_bootstrap(state: tauri::State<'_, AppState>) -> Bootstrap {
    log::info!("bootstrap requested by webview");
    let mut hist = state.history_snapshot();
    hist.truncate(200);
    Bootstrap {
        version: env!("CARGO_PKG_VERSION").to_string(),
        config: state.config_snapshot(),
        dicts: state.dicts.list(),
        history: hist,
        paused: crate::win::hook::is_paused(),
    }
}

#[tauri::command]
pub async fn cmd_lookup(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    word: String,
    source: Option<String>,
) -> Result<LookupResponse, String> {
    let started = Instant::now();
    let word = clean_word(&word);
    if word.is_empty() {
        return Err("没有可查询的内容".into());
    }
    let max = state.config_snapshot().selection.max_chars.max(8);
    if word.chars().count() > max * 4 {
        return Err("内容过长，已跳过查询".into());
    }

    let store = state.dicts.clone();
    let target = word.clone();
    let local = tauri::async_runtime::spawn_blocking(move || store.lookup(&target))
        .await
        .map_err(|e| e.to_string())?;

    let dict_name = local
        .hits
        .first()
        .map(|h| h.name.clone())
        .unwrap_or_default();
    state.push_history(&word, source.as_deref().unwrap_or("manual"), &dict_name);
    let _ = app.emit("history-changed", ());

    Ok(LookupResponse {
        word,
        elapsed_ms: started.elapsed().as_millis() as u64,
        local,
    })
}

#[tauri::command]
pub async fn cmd_dict_add(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<Vec<DictInfo>, String> {
    let file = PathBuf::from(&path);
    if !file.is_file() {
        return Err("文件不存在".into());
    }
    let is_mdx = file
        .extension()
        .map(|e| e.eq_ignore_ascii_case("mdx"))
        .unwrap_or(false);
    if !is_mdx {
        return Err("请选择 .mdx 词典文件".into());
    }

    let id = dict_id(&path);
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "未命名词典".into());
    {
        let Ok(mut cfg) = state.config.lock() else {
            return Err("配置繁忙，请重试".into());
        };
        if cfg.dicts.iter().any(|d| d.id == id) {
            return Err("这本词典已经添加过了".into());
        }
        cfg.dicts.push(DictItem {
            id: id.clone(),
            path: path.clone(),
            name: stem,
            enabled: true,
            mdd: None,
            entry_count: 0,
            size: 0,
        });
    }

    // 打开一次以读取词典名 / 词条数（大词典可能要几秒，放阻塞线程）
    let probe_path = file.clone();
    let probe = tauri::async_runtime::spawn_blocking(move || Dict::probe(&probe_path))
        .await
        .ok()
        .flatten();

    if let Some(probe) = probe {
        if let Ok(mut cfg) = state.config.lock() {
            if let Some(item) = cfg.dicts.iter_mut().find(|d| d.id == id) {
                item.name = probe.name;
                item.entry_count = probe.entry_count;
                item.size = probe.size;
                item.mdd = probe.mdd;
            }
        }
    }

    let cfg = state.config_snapshot();
    state.dicts.sync(&cfg.dicts);
    state.persist(&state.dir())?;
    let _ = app.emit("dicts-changed", ());
    Ok(state.dicts.list())
}

#[tauri::command]
pub fn cmd_dict_remove(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<DictInfo>, String> {
    {
        let Ok(mut cfg) = state.config.lock() else {
            return Err("配置繁忙，请重试".into());
        };
        cfg.dicts.retain(|d| d.id != id);
    }
    let cfg = state.config_snapshot();
    state.dicts.sync(&cfg.dicts);
    state.persist(&state.dir())?;
    let _ = app.emit("dicts-changed", ());
    Ok(state.dicts.list())
}

#[tauri::command]
pub fn cmd_dict_toggle(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<Vec<DictInfo>, String> {
    {
        let Ok(mut cfg) = state.config.lock() else {
            return Err("配置繁忙，请重试".into());
        };
        if let Some(item) = cfg.dicts.iter_mut().find(|d| d.id == id) {
            item.enabled = enabled;
        }
    }
    let cfg = state.config_snapshot();
    state.dicts.sync(&cfg.dicts);
    state.persist(&state.dir())?;
    let _ = app.emit("dicts-changed", ());
    Ok(state.dicts.list())
}

#[tauri::command]
pub fn cmd_dict_move(
    _app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    to: usize,
) -> Result<Vec<DictInfo>, String> {
    {
        let Ok(mut cfg) = state.config.lock() else {
            return Err("配置繁忙，请重试".into());
        };
        if let Some(from) = cfg.dicts.iter().position(|d| d.id == id) {
            let item = cfg.dicts.remove(from);
            let target = to.min(cfg.dicts.len());
            cfg.dicts.insert(target, item);
        }
    }
    let cfg = state.config_snapshot();
    state.dicts.sync(&cfg.dicts);
    state.persist(&state.dir())?;
    Ok(state.dicts.list())
}

/// 重新打开词典：用于外部替换文件后刷新词条数 / 清掉上一次的解析错误
#[tauri::command]
pub async fn cmd_dict_refresh(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<DictInfo>, String> {
    let Some(rt) = state.dicts.runtime(&id) else {
        return Err("词典不存在".into());
    };
    rt.invalidate();
    let path = rt.path.clone();
    let probe = tauri::async_runtime::spawn_blocking(move || Dict::probe(&path))
        .await
        .ok()
        .flatten();
    if let Some(probe) = probe {
        if let Ok(mut cfg) = state.config.lock() {
            if let Some(item) = cfg.dicts.iter_mut().find(|d| d.id == id) {
                item.name = probe.name;
                item.entry_count = probe.entry_count;
                item.size = probe.size;
                item.mdd = probe.mdd;
            }
        }
        let cfg = state.config_snapshot();
        state.dicts.sync(&cfg.dicts);
        state.persist(&state.dir())?;
    }
    let _ = app.emit("dicts-changed", ());
    Ok(state.dicts.list())
}

#[tauri::command]
pub fn cmd_config_set(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    config: AppConfig,
) -> Result<AppConfig, String> {
    if config.selection.hotkey && hotkey::parse_combo(&config.selection.hotkey_combo).is_none() {
        return Err("快捷键格式不正确，示例：Ctrl+Alt+D".into());
    }
    let previous = state.config_snapshot();
    let cfg = state.set_config(config);
    state.persist(&state.dir())?;

    if previous.online.proxy != cfg.online.proxy {
        net::set_proxy(&cfg.online.proxy);
    }

    if previous.autostart != cfg.autostart {
        if let Err(err) = autostart::set(cfg.autostart) {
            log::warn!("autostart failed: {err}");
        }
    }
    hotkey::sync(&app);
    let _ = app.emit("config-changed", &cfg);
    Ok(cfg)
}

#[tauri::command]
pub fn cmd_autostart_set(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    autostart::set(enabled)?;
    if let Ok(mut cfg) = state.config.lock() {
        cfg.autostart = enabled;
    }
    state.persist(&state.dir())?;
    let _ = app.emit("config-changed", state.config_snapshot());
    Ok(())
}

#[tauri::command]
pub fn cmd_history_clear(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    if let Ok(mut guard) = state.history.lock() {
        guard.clear();
    }
    history::save(&state.dir(), &[])?;
    let _ = app.emit("history-changed", ());
    Ok(())
}

#[tauri::command]
pub fn cmd_history_delete(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    at: u64,
) -> Result<(), String> {
    let snapshot = {
        let Ok(mut guard) = state.history.lock() else {
            return Err("历史记录繁忙".into());
        };
        guard.retain(|item| item.at != at);
        guard.clone()
    };
    history::save(&state.dir(), &snapshot)?;
    let _ = app.emit("history-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn cmd_fetch_url(
    url: String,
    referer: Option<String>,
    headers: Option<HashMap<String, String>>,
    method: Option<String>,
    body: Option<String>,
) -> Result<FetchResult, String> {
    net::fetch_text(&url, referer, headers, method, body).await
}

#[tauri::command]
pub fn cmd_popup_hide(app: AppHandle) {
    popup::hide(&app);
}

#[tauri::command]
pub fn cmd_popup_pin(state: tauri::State<'_, AppState>, pinned: bool) {
    state.pinned.store(pinned, Ordering::Relaxed);
}

/// 浮窗挂载后主动取回上一次请求，避免首帧丢事件
#[tauri::command]
pub fn cmd_popup_take(state: tauri::State<'_, AppState>) -> Option<PopupRequest> {
    state
        .pending_popup
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
}

#[tauri::command]
pub fn cmd_popup_resize(app: AppHandle, height: f64) {
    popup::resize(&app, height);
}

#[tauri::command]
pub fn cmd_open_entry(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    word: String,
    source: Option<String>,
) {
    let word = clean_word(&word);
    if word.is_empty() {
        return;
    }
    let pin = state.config_snapshot().selection.pin_on_open;
    popup::show_for(&app, &word, source.as_deref().unwrap_or("manual"), pin);
}

#[tauri::command]
pub fn cmd_open_main(app: AppHandle) {
    popup::show_main(&app);
}

/// 弹出系统文件选择框（阻塞在独立线程，避免卡住 IPC）
#[tauri::command]
pub async fn cmd_pick_mdx() -> Option<String> {
    tauri::async_runtime::spawn_blocking(crate::win::system::pick_mdx)
        .await
        .ok()
        .flatten()
}

#[tauri::command]
pub fn cmd_open_url(url: String) {
    let _ = crate::win::system::open_url(&url);
}

/// 读取 mdd 里的图片/音频，回传 base64 交前端转 data / blob URL
#[tauri::command]
pub async fn cmd_read_asset(
    state: tauri::State<'_, AppState>,
    dict_id: String,
    path: String,
) -> Result<Option<String>, String> {
    use base64::Engine;
    let store = state.dicts.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || store.resource(&dict_id, &path))
        .await
        .map_err(|e| e.to_string())?;
    Ok(bytes.map(|data| base64::engine::general_purpose::STANDARD.encode(data)))
}

/// 前端异常回传：没有 devtools 的 release 包里，这是唯一的排查手段
#[tauri::command]
pub fn cmd_log(level: String, message: String) {
    match level.as_str() {
        "error" => log::error!("[web] {message}"),
        "warn" => log::warn!("[web] {message}"),
        _ => log::info!("[web] {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_selection() {
        assert_eq!(clean_word("  hello \n world  "), "hello world");
        assert_eq!(clean_word("“苹果”"), "苹果");
        assert_eq!(clean_word("   "), "");
    }
}
