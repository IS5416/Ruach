# Ruach

> *Breathe into the void.* （向虚空呼入气息）

个人 Markdown 编辑器，为沉浸式写作而生。Tauri 2 + React 19 + Rust。

## 定位

- 混合场景：长文写作 / 知识笔记 / 日常记录
- `.md` 文件为真源，SQLite 侧车索引（标签、双向链接、全文搜索）
- 源码编辑为主，预览可切换、可分屏
- 三套主题：暖纸（默认）/ 冷石 / 墨夜
- 非 MVP 路线：模块骨架按终版设计，接口先行，实现分步

设计规格见 [docs/superpowers/specs/2026-08-03-ruach-editor-design.md](docs/superpowers/specs/2026-08-03-ruach-editor-design.md)，概念哲学见 `concept.md`。

## 开发命令

```bash
npm run tauri dev       # 全栈开发（Vite + Rust 热重载）
npm run dev             # 仅前端（http://localhost:1420）
npm run build           # typecheck + 前端构建
npm run tauri build     # 发布构建
cargo test              # Rust 服务层测试（在 src-tauri 下）
```

无测试套件时跳过 `npm run build` 的 tsc 检查。

## 架构速览

- **前端** `src/`：CodeMirror 6 编辑器、Zustand 状态、自研微组件。只经 IPC 与 Rust 通信。
- **后端** `src-tauri/`：Rust 服务层（Document / Index / Search / Attachment / Render / Export / Vault / Window / Config），IPC 边界即服务边界。crate lib 名 `ruach_lib`。
- **存储**：Vault 内 `.ruach/index.db`（rusqlite + FTS5 trigram 中文检索）。
- **渲染**：comrak 单引擎（预览与导出共用），原始 HTML 默认丢弃。

## 开发约定

- 提交信息：`type: description`（feat/fix/docs/chore/refactor...）
- 阶段验收门：`cargo test` 全绿
- 命令签名一律 `Result<T, AppError>`，前端不裸 try/catch
