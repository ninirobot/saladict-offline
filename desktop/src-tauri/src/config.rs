use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn default_true() -> bool {
    true
}

fn default_hotkey() -> String {
    "Ctrl+Alt+D".into()
}

fn default_theme() -> String {
    "auto".into()
}

fn default_font_scale() -> f32 {
    1.0
}

fn default_popup_width() -> u32 {
    388
}

fn default_history_limit() -> usize {
    500
}

fn default_engines() -> Vec<String> {
    // 默认只开三个必能用的；其余引擎在设置里按需勾选
    ["youdao", "bing", "baidu"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// 一本已登记的词库
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DictItem {
    pub id: String,
    pub path: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub mdd: Option<String>,
    #[serde(default)]
    pub entry_count: u64,
    #[serde(default)]
    pub size: u64,
}

/// 取词（划词）设置
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SelectionConfig {
    /// 鼠标拖选后自动查询
    #[serde(default = "default_true")]
    pub drag: bool,
    /// 全局快捷键取词
    #[serde(default = "default_true")]
    pub hotkey: bool,
    #[serde(default = "default_hotkey")]
    pub hotkey_combo: String,
    /// 少于此字符数不查询
    #[serde(default = "default_min_chars")]
    pub min_chars: usize,
    /// 多于此字符数视为段落，不查询
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
    /// 划词后自动置顶浮窗
    #[serde(default)]
    pub pin_on_open: bool,
}

fn default_min_chars() -> usize {
    1
}

fn default_max_chars() -> usize {
    80
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            drag: true,
            hotkey: true,
            hotkey_combo: default_hotkey(),
            min_chars: default_min_chars(),
            max_chars: default_max_chars(),
            pin_on_open: false,
        }
    }
}

/// 界面设置
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UiConfig {
    /// auto | light | dark
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_font_scale")]
    pub font_scale: f32,
    #[serde(default = "default_popup_width")]
    pub popup_width: u32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            font_scale: default_font_scale(),
            popup_width: default_popup_width(),
        }
    }
}

/// 在线词典/翻译设置
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnlineConfig {
    #[serde(default = "default_engines")]
    pub enabled: Vec<String>,
    /// 长时间运行后自动降级为只查本地词库
    #[serde(default = "default_true")]
    pub network: bool,
    /// 可选代理，例如 http://127.0.0.1:7890 或 socks5://127.0.0.1:1080
    #[serde(default)]
    pub proxy: String,
}

impl Default for OnlineConfig {
    fn default() -> Self {
        Self {
            enabled: default_engines(),
            network: true,
            proxy: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub dicts: Vec<DictItem>,
    pub selection: SelectionConfig,
    pub ui: UiConfig,
    pub online: OnlineConfig,
    /// 机器翻译引擎凭据，键为 `engineId.field`
    pub engine_keys: std::collections::HashMap<String, String>,
    pub history_limit: usize,
    pub autostart: bool,
    /// 已经跑过一次首启引导
    pub onboarded: bool,
}

impl AppConfig {
    fn with_defaults() -> Self {
        Self {
            dicts: Vec::new(),
            selection: SelectionConfig::default(),
            ui: UiConfig::default(),
            online: OnlineConfig::default(),
            engine_keys: std::collections::HashMap::new(),
            history_limit: default_history_limit(),
            autostart: false,
            onboarded: false,
        }
    }
}

pub fn config_path(dir: &Path) -> PathBuf {
    dir.join("config.json")
}

/// 读取配置；文件缺失或损坏时回落到默认值（损坏文件会备份为 *.bak）
pub fn load(dir: &Path) -> AppConfig {
    let path = config_path(dir);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return AppConfig::with_defaults(),
    };
    match serde_json::from_str::<AppConfig>(&raw) {
        Ok(mut cfg) => {
            if cfg.history_limit == 0 {
                cfg.history_limit = default_history_limit();
            }
            cfg
        }
        Err(err) => {
            log::warn!("config parse failed: {err}");
            let _ = fs::rename(&path, path.with_extension("json.bak"));
            AppConfig::with_defaults()
        }
    }
}

/// 原子写入：先写临时文件再替换，避免断电产生半截 JSON
pub fn save(dir: &Path, cfg: &AppConfig) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = config_path(dir);
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_vec_pretty(cfg).map_err(|e| e.to_string())?;
    {
        let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        file.write_all(&body).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}
