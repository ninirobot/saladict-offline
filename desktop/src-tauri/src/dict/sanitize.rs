//! 词条 HTML 预处理。
//!
//! mdx 词条是「自成一体」的 HTML 片段：链接用 `entry://`，发音用 `sound://`，
//! 图片/音频的相对路径指向 mdd 内部资源。WebView 不认识这些路径，所以这里统一
//! 改写成前端可识别的标记，由前端通过 IPC 取回二进制后转成 data/blob URL：
//!
//! - `href="entry://word"` → `data-entry="word"`
//! - `href="sound://path"` → `data-sound="path"`
//! - `src="pic.png"`       → `data-mdx-src="pic.png"`（并去掉 src，避免相对请求 404）
//!
//! 安全过滤（script / on* 事件）交给前端 DOMPurify 兜底，这里只做必要的剪除。

pub fn rewrite_html(html: &str) -> String {
    let cleaned = strip_scripts(html);
    rewrite_attrs(&cleaned)
}

/// 去掉 `<script ...>...</script>`，避免本地词库注入脚本
fn strip_scripts(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0usize;
    while let Some(start) = lower[cursor..].find("<script") {
        let start = cursor + start;
        out.push_str(&html[cursor..start]);
        match lower[start..].find("</script>") {
            Some(end) => cursor = start + end + "</script>".len(),
            None => {
                cursor = html.len();
                break;
            }
        }
    }
    out.push_str(&html[cursor.min(html.len())..]);
    out
}

fn rewrite_attrs(html: &str) -> String {
    let bytes = html.as_bytes();
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len() + 64);
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        let Some((name_start, name)) = find_attr(&lower, cursor) else {
            out.push_str(&html[cursor..]);
            break;
        };
        let name_end = name_start + name.len();
        let mut p = name_end;
        while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
            p += 1;
        }
        if p >= bytes.len() || bytes[p] != b'=' {
            out.push_str(&html[cursor..name_end]);
            cursor = name_end;
            continue;
        }
        p += 1;
        while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
            p += 1;
        }
        if p >= bytes.len() {
            out.push_str(&html[cursor..]);
            break;
        }
        let quote = bytes[p];
        if quote != b'"' && quote != b'\'' {
            out.push_str(&html[cursor..p]);
            cursor = p;
            continue;
        }
        let value_start = p + 1;
        let Some(rel_end) = html[value_start..].find(quote as char) else {
            out.push_str(&html[cursor..]);
            break;
        };
        let value_end = value_start + rel_end;
        let value = &html[value_start..value_end];

        out.push_str(&html[cursor..name_start]);
        out.push_str(&render_attr(name, value));
        cursor = value_end + 1;
    }
    out
}

/// 返回 `href=` / `src=` 中最早出现的那一处（必须按位置比较，
/// 否则紧跟其后的 href 会让中间的 img src 被整段原样跳过）
fn find_attr(lower: &str, from: usize) -> Option<(usize, &'static str)> {
    let mut best: Option<(usize, &'static str)> = None;
    for name in ["href", "src"] {
        let mut search = from;
        while let Some(idx) = lower[search..].find(name) {
            let abs = search + idx;
            let prev_ok = abs == 0
                || !matches!(
                    lower.as_bytes()[abs - 1],
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b':' | b'_'
                );
            if prev_ok {
                if best.map(|(pos, _)| abs < pos).unwrap_or(true) {
                    best = Some((abs, name));
                }
                break;
            }
            search = abs + name.len();
        }
    }
    best
}

fn render_attr(name: &str, value: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() {
        return format!("{name}=\"\"");
    }
    let lower = raw.to_ascii_lowercase();

    if let Some(target) = lower.strip_prefix("entry://") {
        let word = &raw[raw.len() - target.len()..];
        return format!("href=\"#\" data-entry=\"{}\"", escape_attr(word));
    }
    if let Some(target) = lower.strip_prefix("sound://") {
        let path = &raw[raw.len() - target.len()..];
        return format!("href=\"#\" data-sound=\"{}\"", escape_attr(path));
    }
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("data:")
        || lower.starts_with("mailto:")
        || lower.starts_with('#')
    {
        return format!("{name}=\"{}\"", escape_attr(raw));
    }
    if lower.starts_with("javascript:") {
        return "href=\"#\"".to_string();
    }

    // 相对路径 → 交给前端按 mdd 资源取回
    let path = raw.trim_start_matches('/').trim_start_matches('\\');
    if name == "src" {
        format!("data-mdx-src=\"{}\"", escape_attr(path))
    } else {
        format!("data-mdx-href=\"{}\" href=\"#\"", escape_attr(path))
    }
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_entry_links() {
        let out = rewrite_html(r#"<a href="entry://apple">apple</a>"#);
        assert!(out.contains("data-entry=\"apple\""), "{out}");
        assert!(!out.contains("entry://"));
    }

    #[test]
    fn marks_media_src_for_ipc() {
        let out = rewrite_html(r#"<img src="pic/画面.png">"#);
        assert!(out.contains("data-mdx-src=\"pic/画面.png\""), "{out}");
        assert!(!out.contains("<img src="), "{out}");
    }

    #[test]
    fn rewrites_src_before_later_href() {
        let html = r#"<p><img src="a.png"></p><p><a href="entry://b">b</a></p>"#;
        let out = rewrite_html(html);
        assert!(out.contains("data-mdx-src=\"a.png\""), "{out}");
        assert!(out.contains("data-entry=\"b\""), "{out}");
    }

    #[test]
    fn converts_sound_links() {
        let out = rewrite_html(r#"<a href="sound://apple.mp3">play</a>"#);
        assert!(out.contains("data-sound=\"apple.mp3\""), "{out}");
    }

    #[test]
    fn strips_scripts_and_javascript() {
        let out = rewrite_html(r#"<script>alert(1)</script><a href="javascript:void(0)">x</a>"#);
        assert!(!out.contains("script"));
        assert!(out.contains("href=\"#\""));
    }

    #[test]
    fn leaves_absolute_urls() {
        let html = r#"<img src="https://a.b/c.png">"#;
        assert_eq!(rewrite_html(html), html);
    }

    #[test]
    fn keeps_relative_anchors_inert() {
        let out = rewrite_html(r#"<a href="other.htm">x</a>"#);
        assert!(out.contains("data-mdx-href=\"other.htm\""), "{out}");
    }
}
