# Ruach 开发路线图（P0-P8）

> 依据：`docs/superpowers/specs/2026-08-03-ruach-editor-design.md`
> 原则：非 MVP，接口先行，实现分步。每阶段验收门：`cargo test` 全绿 + 该阶段功能闭环。
>
> **状态：P0-P8 全部完成（2026-08-05）。** 遗留：快照实现（接口+表已就绪）、文件 watcher、双向链接面板、关系图视图、1-2 字搜索已用 LIKE 兜底。

## 阶段总览

| 阶段 | 主题 | 核心交付 | 验收 |
|---|---|---|---|
| P0 | 骨架 | Rust 服务层模块 + AppError + IPC 注册 + SQLite 初始化 + 前端壳 | 空库初始化成功，`cargo test` 绿，`npm run build` 过 |
| P1 | 文档循环 | DocumentService 读写/自动保存/冲突/恢复 + 编辑器接入 | 开存改回完整闭环 + 冲突拒绝 |
| P2 | Vault 与索引 | VaultService 文件树 + IndexService 懒索引 + 标签/链接提取 | 树渲染正确 + 索引一致 |
| P3 | 预览 | RenderService comrak + 编辑/预览切换 + 分屏 + 沉浸态 | 四布局态可切换，GFM 渲染正确 |
| P4 | 附件 | AttachmentService 粘贴存图 + 路径解析 | 截图→插入→预览闭环 |
| P5 | 搜索 | SearchService FTS5 trigram + 命令面板 | 中文子串检索可用，Ctrl+P 闭环 |
| P6 | 多窗口 | WindowManager 会话 + 事件同步 | 双窗口编辑一致 |
| P7 | 主题 | 暖纸/冷石/墨夜 + 排版预设 + 设置面板 | 三主题切换生效，设置持久化 |
| P8 | 导出与快照 | ExportService PDF/HTML + SnapshotService 接口 | 导出产物正确，快照接口定义完整 |

## 阶段依赖

```
P0 → P1 → P2 → P3 → P4 → P5 → P6 → P7 → P8
             ↘ P2 为 P4/P5/P6 前置
```

## 各阶段任务分解

### P0 骨架（本期执行）

**Rust（src-tauri/src/）**
- `error.rs` — AppError 枚举 + Display + Serialize（code/message）
- `services/mod.rs` — 服务模块统一导出
- `services/config.rs` — ConfigService：应用设置 JSON 读写（路径来自 tauri AppHandle path API）
- `services/db.rs` — Database：rusqlite 连接管理、schema 迁移（user_version=1）
- `services/document.rs` — DocumentService：接口签名定义 + 骨架（open/save）
- `services/index.rs` — IndexService：接口签名（index_file/reindex）
- `services/search.rs` — SearchService：接口签名（query）
- `services/attachment.rs` — AttachmentService：接口签名（paste）
- `services/render.rs` — RenderService：接口签名（render_markdown）
- `services/vault.rs` — VaultService：接口签名（open_vault/scan）
- `services/window.rs` — WindowManager：接口签名（create_window）
- `services/export.rs` — ExportService：接口签名（export_pdf/export_html）
- `state.rs` — AppState（数据库 + 服务句柄）
- `commands.rs` — 所有命令注册表
- `lib.rs` — 组装 Builder

**前端（src/）**
- `lib/ipc.ts` — 类型化 invoke 封装
- `lib/error.ts` — 前端错误层
- `stores/` — vaultStore / docStore / editorStore / themeStore / uiStore 骨架
- `components/` — 微组件占位（Button）
- `modules/` — editor / preview / tree / command / tabs / theme / settings / search 目录占位
- `theme/` — CSS variables + 三主题 token 文件
- `App.tsx` — 壳：顶栏 + 文件树占位 + 编辑区占位

**依赖**：rusqlite(bundled)、thiserror、comrak、zustand

### P1 文档循环

- `doc_open(rel_path)`：读文件 → 返回 `{content, meta}` → 广播 `doc:opened`
- `doc_save(rel_path, content, cursor)`：mtime 冲突检测 → write-temp-then-rename → session 副本 → 广播 `doc:changed`
- 前端 editor 模块：CodeMirror 接入、1.5s 防抖保存、状态恢复区
- 测试：保存/冲突/恢复用例

### P2 Vault 与索引

- `vault_open(path)` / `vault_scan()`：目录遍历 → files 表 upsert（mtime 增量）
- `index_file(rel_path)`：`#tag`、`[[目标]]` regex 提取 → tags/links 表 + FTS 行
- 前端 tree 模块：虚拟滚动文件树、展开态、折叠/切换
- 测试：标签链接提取、增量重扫用例

### P3 预览

- `render_markdown(content)`：comrak → HTML（unsafe 关）
- 前端 preview 模块：沙箱 iframe + 样式注入；四布局态切换逻辑
- 测试：GFM 用例集（表格/删除线/代码块/HTML 丢弃）

### P4 附件

- `attach_paste(data)`：命名（`<ts>-<rand>.<ext>`）→ `<vault>/attachments/` → 相对路径回传
- 前端：粘贴/拖放监听、光标插入 `![](...)`、预览相对路径解析
- 测试：命名去重、路径改写用例

### P5 搜索

- `search_query(q)`：FTS5 trigram 标题加权
- 前端 command 模块：命令注册表 + 模糊搜索面板 Ctrl+P
- 测试：中文子串检索用例

### P6 多窗口

- `window_create(rel_path)`：新 WebviewWindow + 会话
- 事件总线：`doc:changed` 跨窗口广播订阅
- 测试：会话状态一致性用例

### P7 主题

- 三主题 token（暖纸/冷石/墨夜）+ 排版预设（衬线/行距/页宽）
- settings 面板 + ConfigService 持久化
- 测试：token 完整性、设置读写用例

### P8 导出与快照

- `export_html(rel_path)`：渲染管线 → HTML 文件；`export_pdf(rel_path)`：打印管线
- SnapshotService 接口定义（快照表预留，实现后置）
- 测试：导出产物断言

## 执行约定

- 每阶段内 TDD：先测试后实现，`cargo test` 全绿提交
- 提交信息：`type: description` 英文，无 co-author
- 每个服务一个 `tests/` 文件
