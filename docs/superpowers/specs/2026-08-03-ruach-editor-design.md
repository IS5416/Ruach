# Ruach 编辑器设计规格

> 日期：2026-08-03
> 状态：已定稿（用户逐段评审通过）
> 配套概念文档：`concept.md`（哲学定位）

## 一、产品定位

个人 Markdown 编辑器，Tauri 2 + React 19 构建。为沉浸式写作而生：长文写作、知识笔记、日常记录混合场景。强调留白、节奏与存在感——"写作是向虚空呼入气息的行为"。

## 二、需求决策（苏格拉底流程锁定）

| 维度 | 决策 |
|---|---|
| 场景 | 混合：长文 / 知识笔记 / 日记 |
| 存储 | `.md` 文件为真源 + SQLite 侧车（派生数据），外部目录同步，不内置云同步 |
| 编辑范式 | 源码为主 + 顶栏编辑/预览切换 + 按钮分屏展开侧栏预览 |
| 知识层 | 文件树 + 全文搜索 + 标签（`#tag`）+ 双向链接（`[[目标]]`） |
| 图片 | 粘贴即存附件目录 + 外链（markdown 原生支持） |
| 周边 | 导出 PDF/HTML、命令面板 Ctrl+P、主题系统、多窗口/多标签 |
| 保存 | 1.5s 防抖自动保存 + mtime 冲突检测 + session 崩溃恢复 + 快照接口留后 |
| 设计原则 | 非 MVP：模块骨架按终版设计，接口先行，实现分步 |

## 三、总体架构

```
┌─────────────────────────────────────────────────┐
│                  Webview (React)                │
│  Editor(CodeMirror)  Preview  FileTree  Search  │
│  Tabs/Windows  Theme  Settings  CommandPalette  │
│  Zustand 状态层        ┌───────┐                │
│                        │ IPC 桥 │  invoke/event │
└────────────────────────┴───┬───┴────────────────┘
                             │
┌────────────────────────────┴────────────────────┐
│              Tauri 核心 (Rust)                  │
│  DocumentService  IndexService  SearchService   │
│  AttachmentService  RenderService  ExportService│
│  VaultService  WindowManager  ConfigService     │
│  ┌────────────────────────────────────────────┐ │
│  │ SQLite 侧车 (rusqlite bundled)             │ │
│  │ .ruach/index.db                            │ │
│  └────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

### 3.1 架构要点

1. **服务边界即 IPC 边界**：每个 Service 一组 `#[tauri::command]`（`doc_open`、`doc_save`、`index_reindex`、`search_query`、`attach_paste`…），前端只消费接口。服务间在 Rust 内部直接调用，不走 IPC。
2. **单一 Markdown 引擎**：comrak（GFM 表格/删除线/自动链接），Rust 侧渲染 HTML。预览与导出共用管线，前端零渲染逻辑。`unsafe_` 默认关 → 原始 HTML 默认丢弃，安全基线免费。
3. **编辑内核**：CodeMirror 6，前端独立模块，只负责源码编辑与语法高亮，不懂文件、不懂保存。
4. **SQLite 侧车**：rusqlite（bundled 特性），同步 API + Tauri 线程池，无 async 负担。位于 `<vault>/.ruach/index.db`。
5. **全文搜索**：FTS5 trigram 分词器（中文子串检索友好，区别于 unicode61）。
6. **事件总线**：Rust `emit` 广播 `doc:opened` / `doc:changed` / `index:updated`，多窗口一致性靠它；前端订阅。
7. **设置与数据分离**：应用设置（主题、排版、窗口状态）存应用数据目录 JSON（ConfigService），不进侧车；侧车只存 Vault 派生数据。一个 Vault 一个侧车。
8. **窗口即文档会话**：每窗口绑定一个文档状态，新窗口 = 新会话；关闭时未保存内容进 session 表，重启进恢复区。

### 3.2 技术选型

| 层 | 选型 | 理由 |
|---|---|---|
| 桌面壳 | Tauri 2（已有） | 既定 |
| 编辑内核 | CodeMirror 6 | 源码编辑王者，扩展生态全，主题可控 |
| Markdown 渲染 | comrak (Rust) | GFM 全支持，单引擎，安全默认 |
| SQLite | rusqlite bundled | 同步、零 async 负担、FTS5 内置 |
| 中文检索 | FTS5 trigram | 任意子串可搜 |
| 前端状态 | Zustand | 轻、无样板 |
| UI 组件 | 自研微组件，不引框架 | 留白调性需完全控制，避免风格污染 |
| 日志 | tracing + tauri-plugin-log | 标准组合 |

## 四、SQLite 侧车 Schema

`<vault>/.ruach/index.db`，`PRAGMA user_version = 1`。派生数据均以 Vault 相对路径为键（Vault 移动/外部同步后仍自洽）。

```sql
CREATE TABLE files (
  rel_path   TEXT PRIMARY KEY,      -- "notes/风的形状.md"
  title      TEXT NOT NULL,         -- 首行 # 标题，无则取文件名
  mtime      INTEGER NOT NULL,      -- 索引时的文件修改时间（增量重扫依据）
  size       INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE tags (
  rel_path TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  tag      TEXT NOT NULL,
  PRIMARY KEY (rel_path, tag)
);

CREATE TABLE links (                 -- 双向链接：[[目标]] 语法
  src_path TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  dst_path TEXT NOT NULL,           -- 未解析目标也留（red link 后置）
  label    TEXT,
  PRIMARY KEY (src_path, dst_path)
);

CREATE TABLE attachments (
  rel_path TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  name     TEXT NOT NULL,           -- 附件目录内文件名
  orig_name TEXT,                   -- 粘贴时原始名
  created_at INTEGER NOT NULL,
  PRIMARY KEY (rel_path, name)
);

CREATE TABLE recent (
  rel_path   TEXT PRIMARY KEY REFERENCES files(rel_path) ON DELETE CASCADE,
  opened_at  INTEGER NOT NULL
);

CREATE TABLE sessions (              -- 崩溃恢复缓冲（应用级，重启清理）
  doc_key   TEXT PRIMARY KEY,       -- 相对路径 或 ":untitled:<ts>"（未命名草稿）
  content   TEXT NOT NULL,
  cursor    INTEGER,
  updated_at INTEGER NOT NULL
);

CREATE VIRTUAL TABLE docs_fts USING fts5(
  rel_path UNINDEXED, title, body, tokenize='trigram'
);
```

**索引策略：懒索引**——打开文件时索引该文件；启动时仅对 mtime 变化文件快速重扫；`index:reindex` 全量重建兜底。骨架期不做文件 watcher（VaultService 留接口，二期加）。

## 五、关键流程

1. **打开文档** `doc_open(rel_path)`：读文件 → 懒索引（`#tag` / `[[目标]]` regex 提取）→ 返回内容+元数据 → 前端进 CodeMirror。广播 `doc:opened`。
2. **保存**：输入防抖 1.5s → `doc_save`（mtime 冲突检测：盘上比会话新 → 拒绝覆盖，回 `FileChanged` 错误，前端提示合并）→ 增量索引 → 广播 `doc:changed`。
3. **崩溃恢复**：`doc_save` 同时写 session 表副本；启动时 session 非空 → 恢复区提示 → 用户选恢复/丢弃。
4. **图片粘贴** `attach_paste(data_url)`：去重命名（时间戳+短随机）写入 `<vault>/attachments/` → 返回相对路径 → 光标处插入 `![](attachments/xxx.png)`。预览时相对路径解析为绝对路径供 img 加载。
5. **搜索** `search_query(q)`：FTS5 trigram 标题加权（title 命中优先）→ rel_path 列表 → 命令面板模糊选择 → `doc_open`。
6. **预览** `render_markdown(content)`：comrak HTML → 前端注入沙箱 iframe（无 JS 权限，原始 HTML 已被丢弃）。

## 六、错误处理与测试

- Rust 统一 `AppError` 枚举（`Io` / `NotFound` / `FileChanged` / `Db` / `Parse` / `Vault`…）→ 序列化 `{ code, message }` → 前端统一错误层映射提示。命令签名一律 `Result<T, AppError>`。
- tracing + tauri-plugin-log；`AppError` 实现 `Display` 带上下文。
- Service 层全部 `Result`，不 `unwrap`（测试除外）；写文件 write-temp-then-rename 防半写。
- 测试：Rust 单元测试为主（SQLite 用 `:memory:` 或临时目录）。DocumentService 保存/冲突、IndexService 标签链接提取、SearchService 中文查询、AttachmentService 命名与路径改写、RenderService GFM 用例。前端 vitest 后置。`cargo test` 全绿为阶段验收门。

## 七、前端模块结构

```
src/
  app/          — 启动、窗口会话、状态恢复
  components/   — 自研微组件（Button/Menu/Modal/Icon）
  modules/
    editor/     — CodeMirror 封装、光标/选区、自动保存节流
    preview/    — 沙箱 iframe 预览容器、渲染消息通道
    tree/       — 文件树：虚拟滚动、展开态、右键菜单
    command/    — 命令面板：命令注册表 + 模糊搜索
    tabs/       — 标签页 ↔ 窗口会话映射
    theme/      — 设计 tokens（CSS variables）、主题切换、排版预设
    settings/   — 设置面板
    search/     — 全文搜索入口
  stores/       — Zustand：vaultStore / docStore / editorStore / themeStore / uiStore
  lib/          — 类型化 IPC 封装、错误层、工具
```

### 布局状态（四档，对应呼吸节奏）

1. 默认：顶栏（极窄：文档标题 + 编辑/预览切换 + 分屏按钮）+ 左文件树（可折叠）+ 编辑区
2. 预览态：编辑区原地变预览
3. 分屏：左右并排
4. 沉浸书写态：藏树藏栏，居中窄栏，只剩文字

## 八、主题系统

三套主题共存可切换，CSS variables tokens 驱动：

| 主题 | 底 | 文字 | 性格 |
|---|---|---|---|
| 暖纸（默认） | 米白 `#f6f1e7` | 棕墨衬线（Georgia/宋体） | 旧书纸尘 |
| 冷石 | 灰白 `#f4f4f3` | 深灰无衬线（Inter/思源黑体） | 北欧克制 |
| 墨夜 | 深墨 `#211f1b` | 暖白衬线 | 夜间呼吸 |

暗色主题 = 墨夜变体。排版预设（衬线/无衬线、行距、页宽）独立于色板。

## 九、阶段开发计划（概述，细节见实现计划文档）

| 阶段 | 内容 | 验收 |
|---|---|---|
| P0 骨架 | Rust 服务层骨架 + AppError + IPC 桥 + 配置 | 骨架可跑通空库初始化 |
| P1 文档循环 | DocumentService 读写/自动保存/冲突/恢复 + 编辑器接入 | 开存改回完整闭环 |
| P2 Vault 与索引 | VaultService 文件树 + IndexService 懒索引 + 标签/链接提取 | 树渲染 + 索引正确 |
| P3 预览 | RenderService comrak + 编辑/预览切换 + 分屏 + 沉浸态 | 三态切换 |
| P4 附件 | AttachmentService 粘贴存图 + 路径解析 | 截图→插入→预览闭环 |
| P5 搜索 | SearchService FTS5 + 命令面板 | 中文子串检索可用 |
| P6 多窗口 | WindowManager 会话 + 事件同步 | 双窗口一致 |
| P7 主题 | 三主题 + 排版预设 + 设置面板 | 切换生效 |
| P8 导出与快照 | ExportService PDF/HTML + SnapshotService 接口 | 导出产物正确 |

## 十、非目标（本期不做）

- 内置云同步（外部目录同步代替）
- 关系图视图（Graph View）——接口留，实现后置
- 插件系统、移动端
- 版本快照实现（接口留）
- 文件系统 watcher（二期）
- 前端组件测试框架（后置）
