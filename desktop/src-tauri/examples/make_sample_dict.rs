//! 生成一份用于冒烟测试的示例词库（含 mdd 图片与 entry:// 互链）。
//!
//! 用法：
//!   cargo run --example make_sample_dict -- "D:\temp\saladict-sample"
//!
//! 会写出 sample.mdx / sample.mdd，并打印可直接放进 config.json 的词典条目。

use std::path::PathBuf;

use mdictlib::{MddBuilder, MdxBuilder, WriteEncoding, WriteOptions};

const ICON: &[u8] = include_bytes!("../icons/32x32.png");

fn dict_id(path: &str) -> String {
    // 与 saladict_lib::dict::dict_id 保持一致（FNV-1a 64）
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.to_lowercase().as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("d{hash:016x}")
}

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("需要输出目录参数"));
    std::fs::create_dir_all(&dir).expect("create dir");

    let options = WriteOptions::new().with_encoding(WriteEncoding::Utf8);

    let mut builder = MdxBuilder::with_options(options.clone());
    builder.header_attribute("Title", "示例词典").unwrap();
    builder
        .add_entry(
            "apple",
            r#"<h2 style="color:#2b57c9">apple</h2>
<p style="color:#8a93a1">/ˈæpl/ <a href="sound://apple.mp3">🔊 发音</a></p>
<ol><li><b>n.</b> 苹果；苹果树</li><li>苹果公司（Apple Inc.）</li></ol>
<p>例句：An <b>apple</b> a day keeps the doctor away.</p>
<p><img src="apple.png" alt="apple"></p>
<p>相关词条：<a href="entry://pear">pear</a> · <a href="entry://banana">banana</a></p>"#,
        )
        .unwrap();
    builder
        .add_entry(
            "pear",
            r#"<h2>pear</h2><p>n. 梨</p><p><a href="entry://apple">← 回到 apple</a></p>"#,
        )
        .unwrap();
    builder
        .add_entry("banana", r#"<h2>banana</h2><p>n. 香蕉</p>"#)
        .unwrap();

    let mdx_path = dir.join("sample.mdx");
    let mut mdx_file = std::fs::File::create(&mdx_path).unwrap();
    let summary = builder.finish(&mut mdx_file).unwrap();

    let mut mdd_builder = MddBuilder::with_options(options);
    mdd_builder.add_resource("\\apple.png", ICON).unwrap();
    let mut mdd_file = std::fs::File::create(dir.join("sample.mdd")).unwrap();
    mdd_builder.finish(&mut mdd_file).unwrap();

    let path = mdx_path.to_string_lossy().replace('\\', "\\\\");
    println!(
        "{{\"id\":\"{}\",\"path\":\"{}\",\"name\":\"示例词典\",\"enabled\":true,\"entryCount\":{},\"size\":{}}}",
        dict_id(&mdx_path.to_string_lossy()),
        path,
        summary.entries,
        std::fs::metadata(&mdx_path).map(|m| m.len()).unwrap_or(0)
    );
}
