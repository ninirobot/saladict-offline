# 沙拉划词 · 桌面版

把「划词查词」做成一个 Windows 小工具：常驻托盘，在任意程序里选中文字就弹出释义。
词典来源有两类——**本地 mdx 词库**与**在线词典/翻译**。整个程序只有 **4.25 MB**（单个 exe，
复用系统 WebView2，不打包 Chromium）。

---

## 一、能做什么

- **划词即查**：鼠标拖选文字松开后自动查询；也支持全局快捷键读取当前选中内容（默认 `Ctrl+Alt+D`）。两种方式都能在设置里单独关闭。
- **本地 mdx 词库**：添加多本 mdx，多本同时查、按顺序展示；支持 `@@@LINK=` 别名重定向、词条内 `entry://` 跳转、`sound://` 发音、同名 `.mdd` 里的图片与音频。
- **在线查词**：内置 **26 个引擎**（20 个词典 + 6 个机器翻译）。请求由 Rust 侧发出，**不受浏览器跨域限制**；需要密钥的翻译引擎在设置里填一次即可；被墙或被 Cloudflare 拦截的站点可在设置里填代理。
- **浮窗**：无边框圆角卡片，跟随光标出现，不抢焦点；可置顶、拖动、`Esc` 关闭、点击窗外自动收起。
- **历史 / 设置 / 词库管理**：主窗口四个页签，改动即时生效（无保存按钮）。
- **托盘常驻**：打开主窗口、剪贴板取词、暂停划词、退出；可选开机自启。

---

## 二、目录结构

```
desktop/
├── package.json / vite.config.ts / tsconfig.json   前端（Preact + TS + Vite）
├── index.html                                      单页入口，按窗口 label 分流
├── src/
│   ├── main.tsx                                    入口：区分 main / popup 窗口
│   ├── api.ts                                      Tauri 命令封装（唯一的 IPC 出口）
│   ├── lookup.ts                                   查询编排：本地词库 + 在线引擎并行
│   ├── util.ts                                     解析/转义/排版/音频/主题，全部共用工具
│   ├── types.ts                                    与 Rust serde 结构一一对应
│   ├── engines/
│   │   ├── index.ts                                Engine 契约 + 注册表 + 并发调度（8s 超时）
│   │   ├── http.ts                                 请求封装（GET/POST、Referer、错误提示）
│   │   ├── dicts.ts                                20 个词典引擎
│   │   └── mt.ts                                   6 个机器翻译引擎（含密钥字段声明）
│   ├── views/                                      Popup / MainWindow / ResultView / EntryView / 面板
│   └── styles/                                     app.css（变量+浮窗+主窗口）、entry.css（词条排版）
├── scripts/
│   ├── gen-icons.py                                纯标准库生成 ico/png 图标
│   └── smoke-ui.ps1                                UI 冒烟：记事本取词 + 截屏
└── src-tauri/
    ├── tauri.conf.json                             窗口、托盘、CSP、图标（关闭安装包打包）
    ├── capabilities/default.json                   最小权限集
    ├── examples/make_sample_dict.rs                生成示例 mdx/mdd（自测用）
    └── src/
        ├── lib.rs                                  装配：托盘、钩子、热键、命令注册
        ├── state.rs                                应用状态 + 全部 Tauri 命令
        ├── config.rs / history.rs / net.rs         配置、历史、HTTP 抓取
        ├── logging.rs                              1MB 滚动文件日志（不记录词条内容）
        ├── tray.rs                                 托盘菜单
        ├── dict/
        │   ├── mdx.rs                              mdictlib 封装：查词、@@@LINK、联想、mdd
        │   ├── sanitize.rs                         词条 HTML 改写（entry:// → data-entry …）
        │   └── mod.rs                              词库管理器：懒加载、按序查询
        └── win/
            ├── hook.rs                             WH_MOUSE_LL 拖选检测 + 取词工作线程
            ├── hotkey.rs                           RegisterHotKey 全局快捷键
            ├── capture.rs                          UIA 优先取词，Ctrl+C 兜底并还原剪贴板
            ├── uia.rs                              UI Automation TextPattern
            ├── popup.rs                            浮窗创建、DPI 定位、夹取、自适应高度
            ├── system.rs                           文件选择框、外部链接
            └── autostart.rs                        开机自启（HKCU Run）
```

---

## 三、构建

一条命令出 exe（自动先构建前端，**不产出安装包**）：

```bash
cd desktop
npm install              # 首次需要
npm run app:build        # → src-tauri/target/release/saladict-desktop.exe（≈ 4.1 MB）
```

等价于在 `src-tauri/` 下执行 `cargo tauri build --no-bundle`。
只需要重新编译 Rust 侧时，`cd src-tauri && cargo build --release` 更快（前提是 `dist/` 已存在）。

开发调试（前端热更新）：

```bash
cd desktop
npm run dev              # 只起 Vite（127.0.0.1:5183）
cd src-tauri && cargo run  # debug 版会加载 devUrl

# 或者一步到位（自动执行 beforeDevCommand）
npm run app:dev
```

> 依赖：Rust 1.77+（msvc 工具链 + Windows SDK）、Node 18+、WebView2 Runtime（Win10/11 通常已内置）。
> 分发时直接拷贝单个 exe 即可，无需安装程序。

---

## 四、使用

| 操作 | 说明 |
|---|---|
| 划词 | 在任意程序里拖动选中文字，松手即弹出浮窗 |
| 快捷键 | 默认 `Ctrl+Alt+D`，可在「设置」里点击胶囊框重新录制 |
| 置顶 | 浮窗右上角图钉；置顶后不会被点击窗外收起 |
| 关闭浮窗 | `Esc` 或右上角 ×，或点窗外任意位置 |
| 词条内跳转 | 点击 `entry://` 链接即查新词；`sound://` 与 mdd 图片自动取回 |
| 添加词库 | 主窗口「词库」→ 添加 mdx；同名 mdd 会自动关联 |
| 托盘 | 打开主窗口 / 剪贴板取词 / 暂停划词 / 退出 |
| 主窗口关闭 | 只是收进托盘，进程继续监听；从托盘「退出」才真正结束 |

数据目录：`%APPDATA%\com.saladict.lite\`

```
config.json    词库列表、划词与热键设置、在线引擎开关、翻译引擎凭据（engineKeys）
history.json   最近查询（默认上限 500，可改）
logs/app.log   只记录错误与关键事件，不记录查询内容；超过 1MB 滚动一份
```

---

## 五、在线引擎

**词典（20）**：有道、必应、剑桥、韦氏、柯林斯、朗文、牛津、Vocabulary、词源(Etymonline)、
沪江、AHD、欧路、Naver、Weblio、Weblio英和、扇贝、人人词典、百度、维基、Urban。

**机器翻译（6）**：谷歌（免密钥）、DeepL、DeepLX、有道翻译、彩云小译、小牛翻译。

默认只启用 **有道、必应、百度** 三个（免密钥、可直接用），其余在设置里勾选。
需要密钥的引擎勾上后会自动出现输入框（声明在 `fields` 里），值存在 `config.json` 的 `engineKeys`。

新增一个引擎只需两步：

```ts
// src/engines/dicts.ts（或 mt.ts）
const example: Engine = {
  id: 'example',
  name: '示例',
  kind: 'dict',
  async search(word, ctx) {
    const doc = parseHtml(await fetchPage('https://example.com/' + encodeURIComponent(word)))
    if (ctx.signal.aborted) return null
    return { title: text(doc, 'h1'), html: renderDefs([{ def: text(doc, '.def') }]) }
  }
}
```

再把它加进同文件末尾的 `DICTS`（或 `MTS`）数组即可，设置面板会自动出现开关。

选择器移植自沙拉查词原有的引擎（`src/components/dictionaries/*​/engine.ts`），
把 React 组件换成了字符串模板，去掉了 redux / i18n / shadow DOM 依赖。

### 未内置的 4 个

| 引擎 | 原因 |
|---|---|
| 必应翻译 | 上游 `edge.microsoft.com/translate/auth` 已下线，网页版 ttranslatev3 需浏览器完整会话 |
| 腾讯翻译君 | 需要腾讯云 TC3-HMAC-SHA256 签名 |
| 火山翻译 | 需要火山引擎 AWS 风格签名 |
| 阿里翻译 | 需要阿里云 HMAC-SHA1 签名（WebCrypto 不提供 SHA-1） |
| Moji 辞书 | 需要其私有接口的专有签名 |

这四家都要在本地实现签名算法（且无法在无密钥环境下验证），为保持体积与可维护性暂未接入；
如果你确实在用其中某个，告诉我，我按单引擎方式补上。

---

## 六、实测数据（本机 Win10/11 + 200% 缩放）

| 指标 | 数值 |
|---|---|
| release exe | **4.25 MB**（开启 LTO + `opt-level=s` + strip，单文件免安装） |
| 前端产物 | app.js ≈ 100 KB + app.css 13 KB（26 个引擎 + 无 UI 组件库） |
| 进程内存 | 空闲约 24 MB 工作集（渲染进程由系统 WebView2 共享） |
| 本地词库查询 | 1–3 ms（含渲染） |
| 浮窗出现 | 首次创建约 300 ms，之后仅 show/定位 |

---

## 七、已知限制

1. **仅 Windows**：取词依赖 Win32（`WH_MOUSE_LL`、RegisterHotKey、UI Automation）。
2. **部分在线词典受网络与站点策略影响**：韦氏/柯林斯/词源在国内网络会被 Cloudflare 判为可疑流量直接 403；维基、谷歌翻译、Urban 需要能出海的网络。这类错误会显示为带 ⚠ 的页签，点击可看到具体原因，不影响本地词库使用。**在「设置 → 网络代理」里填一个代理即可全部恢复**（如 `http://127.0.0.1:7890`），代理改动立即生效，无需重启。
3. **加密 mdx** 不支持（mdictlib 需要 passcode 才能解密，界面上暂未提供输入框）。
4. **mdd 资源走 IPC + base64**：单张图片几十 KB 时无感，超大插图会稍慢；图片会随词条一起加载，滚动时不再二次请求。
5. **划词取词的两种方式**：优先用 UI Automation 读取选区（不碰剪贴板）；不支持 UIA 的程序会退回「模拟 Ctrl+C → 读剪贴板 → 还原原剪贴板」。
6. **不做的事**：Anki / WebDAV 同步、生词本、PDF 嗅探、右键菜单、波形播放器——保持体积与心智负担最小。

---

## 八、自测

```bash
# Rust 单元测试：mdx 解析、@@@LINK、mdd 资源、词条 HTML 改写、快捷键解析、文本清洗
cd src-tauri && cargo test

# 生成一份示例词库（含 mdd 图片与 entry:// 互链）
cargo run --example make_sample_dict -- "D:\temp\saladict-sample"
# 把打印出的 JSON 放进 config.json 的 dicts 数组即可

# UI 冒烟：在记事本里选中一个词，用热键取词并截屏
cd .. && powershell -ExecutionPolicy Bypass -File scripts\smoke-ui.ps1 -Word apple
```

`smoke-ui.ps1` 会把结果截图存到 `%TEMP%\saladict-popup.png` / `saladict-main.png`，
并可从 `%APPDATA%\com.saladict.lite\logs\app.log` 里看到取词命中字符数与浮窗定位参数。
