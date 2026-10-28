use std::path::{Path, PathBuf};

use mdictlib::{Limits, MddFile, MdxFile, OpenOptions};

use super::sanitize::rewrite_html;

const LINK_PREFIX: &str = "@@@LINK=";
const MAX_REDIRECT: usize = 8;

/// 一本已打开的词典（mdx + 同名 mdd）
pub struct Dict {
    pub id: String,
    pub path: PathBuf,
    pub name: String,
    pub entry_count: u64,
    pub size: u64,
    pub mdd_path: Option<PathBuf>,
    mdx: MdxFile,
    mdd: Option<MddFile>,
}

pub struct RawHit {
    pub key: String,
    pub html: String,
}

fn err_text(e: mdictlib::Error) -> String {
    e.to_string()
}

/// 同名 .mdd（大小写不敏感）自动关联，用于图片与原生发音
fn find_mdd(path: &Path) -> Option<PathBuf> {
    let dir = path.parent()?;
    let stem = path.file_stem()?.to_string_lossy().to_lowercase();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        let is_mdd = p
            .extension()
            .map(|ext| ext.eq_ignore_ascii_case("mdd"))
            .unwrap_or(false);
        if !is_mdd {
            continue;
        }
        if let Some(s) = p.file_stem() {
            if s.to_string_lossy().to_lowercase() == stem {
                return Some(p);
            }
        }
    }
    None
}

impl Dict {
    pub fn open(id: String, path: &Path, display_name: Option<String>) -> Result<Self, String> {
        let options = OpenOptions::new().with_limits(Limits::large_dictionary());
        let mdx = MdxFile::open_with_options(path, &options).map_err(err_text)?;
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let entry_count = mdx.len();
        let name = display_name
            .filter(|n| !n.trim().is_empty())
            .or_else(|| {
                mdx.header()
                    .title()
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
            })
            .unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "未命名词典".into())
            });
        let mdd_path = find_mdd(path);
        let mdd = mdd_path.as_ref().and_then(|p| MddFile::open(p).ok());
        Ok(Self {
            id,
            path: path.to_path_buf(),
            name,
            entry_count,
            size,
            mdd_path,
            mdx,
            mdd,
        })
    }

    /// 查词并展开 `@@@LINK=` 重定向（带环检测）
    pub fn lookup(&self, word: &str) -> Result<Option<RawHit>, String> {
        let mut key = word.trim().to_string();
        let mut visited: Vec<String> = Vec::new();
        for _ in 0..MAX_REDIRECT {
            let entry = match self.mdx.lookup(&key).map_err(err_text)? {
                Some(entry) => entry,
                None => return Ok(None),
            };
            let text = entry.text().to_string();
            let real_key = entry.key().to_string();
            match parse_link(&text) {
                Some(target) => {
                    if visited.iter().any(|v| v.eq_ignore_ascii_case(&target)) {
                        break;
                    }
                    visited.push(target.clone());
                    key = target;
                }
                None => {
                    return Ok(Some(RawHit {
                        key: real_key,
                        html: rewrite_html(&text),
                    }))
                }
            }
        }
        Ok(None)
    }

    /// 前缀候选（输入框联想）
    pub fn suggest(&self, prefix: &str, limit: usize) -> Vec<String> {
        match self.mdx.prefix_keys(prefix, limit) {
            Ok(keys) => keys.into_iter().map(|k| k.key().to_string()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// 读取 mdd 内的资源（图片/音频）
    pub fn resource(&self, path: &str) -> Option<Vec<u8>> {
        let mdd = self.mdd.as_ref()?;
        let candidate = normalize_mdd_key(path);
        if let Some(res) = mdd.lookup(&candidate).ok().flatten() {
            return Some(res.bytes().to_vec());
        }
        // 部分词典在 mdd 里带子目录前缀，退化为后缀匹配
        mdd.lookup(&candidate.replace('\\', "/"))
            .ok()
            .flatten()
            .map(|res| res.bytes().to_vec())
    }
}

/// 添加词典时的轻量探测：拿到词典名、词条数、体积与同名 mdd
pub struct Probe {
    pub name: String,
    pub entry_count: u64,
    pub size: u64,
    pub mdd: Option<String>,
}

impl Dict {
    pub fn probe(path: &Path) -> Option<Probe> {
        Dict::open("__probe__".into(), path, None).ok().map(|dict| Probe {
            name: dict.name.clone(),
            entry_count: dict.entry_count,
            size: dict.size,
            mdd: dict
                .mdd_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
        })
    }
}

fn normalize_mdd_key(path: &str) -> String {
    let mut key = path.trim().replace('/', "\\");
    if !key.starts_with('\\') {
        key.insert(0, '\\');
    }
    key
}

/// `@@@LINK=apple` → `apple`
fn parse_link(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    let rest = trimmed.strip_prefix(LINK_PREFIX)?;
    let target = rest.lines().next().unwrap_or("").trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdictlib::{MddBuilder, MdxBuilder, WriteEncoding, WriteOptions};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("saladict-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn build_mdx(path: &Path) {
        let options = WriteOptions::new().with_encoding(WriteEncoding::Utf8);
        let mut builder = MdxBuilder::with_options(options);
        builder.header_attribute("Title", "test dictionary").unwrap();
        builder
            .add_entry(
                "apple",
                "<b>apple</b><a href=\"entry://pear\">pear</a><p><img src=\"apple.png\" alt=\"apple\"></p>",
            )
            .unwrap();
        builder.add_entry("pear", "a fruit").unwrap();
        builder.add_entry("fruit", "@@@LINK=apple").unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        builder.finish(&mut file).unwrap();
    }

    #[test]
    fn reads_entries_and_follows_links() {
        let dir = temp_dir("mdx");
        let path = dir.join("sample.mdx");
        build_mdx(&path);

        let dict = Dict::open("t".into(), &path, None).expect("open mdx");
        assert_eq!(dict.name, "test dictionary");
        assert_eq!(dict.entry_count, 3);

        let hit = dict.lookup("apple").expect("lookup ok").expect("hit");
        assert!(hit.html.contains("data-entry=\"pear\""), "{}", hit.html);
        assert!(hit.html.contains("data-mdx-src=\"apple.png\""), "{}", hit.html);

        let redirected = dict.lookup("fruit").expect("ok").expect("hit");
        assert_eq!(redirected.key, "apple");

        assert!(dict.lookup("missing").expect("ok").is_none());
        assert!(!dict.suggest("ap", 5).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_mdd_resource() {
        let dir = temp_dir("mdd");
        let mdx_path = dir.join("sample.mdx");
        build_mdx(&mdx_path);

        let options = WriteOptions::new().with_encoding(WriteEncoding::Utf8);
        let mut builder = MddBuilder::with_options(options);
        builder.add_resource("\\pic.png", b"PNGDATA").unwrap();
        let mut file = std::fs::File::create(dir.join("sample.mdd")).unwrap();
        builder.finish(&mut file).unwrap();

        let dict = Dict::open("t".into(), &mdx_path, None).expect("open mdx");
        assert!(dict.mdd_path.is_some(), "mdd should be detected");
        assert_eq!(dict.resource("pic.png").as_deref(), Some(&b"PNGDATA"[..]));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
