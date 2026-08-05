# Ruach 全面审查报告

日期：2026-08-05
方法：8 个专家代理并行只读审查（安全 / 正确性 / Rust 质量 / 前端状态 / 前端性能 / 可访问性 / 构建配置 / 规格一致性），关键结论已人工抽查验证。

统计：原始 findings 85 条，跨维度去重合并后约 60 条。Critical 7 条、Warning 28 条、Nit 约 25 条。

---

## Critical（必须尽快修）

### C1. 主题 token 整体失效：`data-theme` 挂载点与 CSS 选择器不匹配
- **文件**：`src/theme/tokens.css:34,50,66` + `src/App.tsx:131`
- 问题：tokens.css 用 `:root[data-theme="warm_paper"]` 等选择器定义全部颜色 token，但 `:root` 只匹配 `<html>`；App.tsx 把 `data-theme` 挂在 `<div className="app">` 上，全仓库无任何地方给 `document.documentElement` 设 data-theme。**三套主题全部不生效**，界面为浏览器默认色（白底黑字），主题切换无效；预览 iframe 的 `themeCss()` 从 documentElement 读 computed style 也拿到空值。
- 修复：App 挂载 effect 中 `document.documentElement.dataset.theme = theme`（推荐），themeCss() 改读实际挂载元素；或选择器改为 `.app[data-theme=...]`。
- 验证状态：已人工确认（grep 全仓库无 dataset.theme 写入）。

### C2. `attach_read` / `export_document` 无路径校验：任意文件读写
- **文件**：`src-tauri/src/commands.rs:164`、`src-tauri/src/services/attachment.rs:96`、`src-tauri/src/services/export.rs:45`
- 问题：`validate_rel_path` 只在 doc_open/doc_save 调用（document.rs:51,89），attach_read 和 export 直接 `vault.join(rel_path)` + fs::read。前端传 `../../.ssh/id_rsa`、`../../.ruach/index.db` 即可任意读（attach_read base64 回传、export 可把任意 UTF-8 文件渲染写出）；export 的 dest_dir 也由前端任意指定，可任意位置写 .html。
- 修复：入口统一校验（抽共享 `resolve_vault_path(vault, rel_path)` 辅助），read/export/index 全过门禁；attach_read 加大小上限（如 50MB）；export 限定 dest_dir 或经 dialog 选择。
- 验证状态：已人工确认（validate_rel_path 仅 2 处调用）。

### C3. `doc_open`/`index_file` 读取侧信道：校验前文件已进 FTS 索引
- **文件**：`src-tauri/src/commands.rs:65`、`src-tauri/src/services/index.rs:51`
- 问题：doc_open 先 `IndexService::index_file`（无校验直接读文件入 docs_fts），之后 DocumentService::open 才拒绝。`doc_open("../../secret.txt")` 报错但内容已入索引，随后 search_query 可检索出任意文本文件内容。
- 修复：doc_open 内校验提前到 index_file 之前；index_file 自身也加 validate_rel_path + 仅接受 .md。

### C4. mtime 冲突检测只有秒级精度：同秒保存静默覆盖丢数据
- **文件**：`src-tauri/src/services/document.rs:64,98,126`（open 基线 / save 冲突比对 / save 返回）
- 问题：全部 `as_secs() as i64` 截断到整秒。双窗口（Ctrl+Shift+N 是设计功能）或外部编辑器与本应用同秒内先后保存，B 的 expected 与磁盘 mtime 相等 → 冲突检查通过 → B 旧内容覆盖 A 新内容，无任何提示。doc:changed 因 B dirty 也不重载。
- 修复：改毫秒精度（`as_millis() as i64`），DocumentMeta.mtime 与前端 docStore 基线同步改。
- 验证状态：已人工确认（document.rs 全部 as_secs）。

### C5. openDoc 无条件覆盖未保存内容 + 全链路无"关闭/切换前 flush"
- **文件**：`src/stores/docStore.ts:36`、`src/modules/tree/index.tsx:120`、`src/modules/editor/index.tsx:76-80`
- 问题：(1) 点树切文档无条件用磁盘内容替换 store，1.5s 防抖窗口内输入静默丢弃，无确认、无 session flush；(2) 全前端无 beforeunload / onCloseRequested 钩子，文件文档从不写 session 恢复缓冲（sessionFlush 只在草稿路径 docStore.ts:81 调用，已确认），关窗/崩溃时防抖窗口内的改动永久丢失——"崩溃恢复"实际只覆盖未命名草稿。
- 修复：openDoc 开头若 dirty 先 await save()（或 flush 再切）；App 加 beforeunload / Rust on_window_event 对 dirty 内容 sessionFlush；修正 RecoveryBanner.tsx:8 失实注释。

### C6. save() 完成后盲目清 dirty：保存飞行中输入的新内容被误标已保存
- **文件**：`src/stores/docStore.ts:79`
- 问题：save 开头捕获 content，resolve 时无条件 `set({ dirty: false })`。输入 v1 → save(v1) 在飞 → 输入 v2 → save resolve 清 dirty → 防抖 effect cleanup 取消 T2 → v2 永不保存/flush，关窗即丢。
- 修复：捕获 savedContent=content，resolve 后仅当 `get().content === savedContent` 才清 dirty。

### C7. Windows 盘符相对路径绕过 validate_rel_path（"C:foo"）+ 含 `..` 子串误杀合法文件名
- **文件**：`src-tauri/src/services/document.rs:24-28`
- 问题：(1) `"C:foo"` 的 is_absolute() 在 Windows 为 false、又不含 `/`、`\`、`..` → 校验通过；`vault.join("C:foo")` 按 std 语义整体替换 → 越界读写（save 还 create_dir_all 其 parent）。测试只覆盖了 `"C:/windows"`，未覆盖 `"C:foo"`。(2) `contains("..")` 子串检查误杀 `my..notes.md`、`a..b/` 等合法名。
- 修复：组件级检查（拒绝 `Component::Prefix(_)` 与 `Component::ParentDir`）；最终防御 canonicalize + starts_with(canonicalize(vault))。

---

## Warning

### 数据完整性
- **W1. save 成功落盘后 DELETE sessions 失败伪装成错误，且基线过期致后续保存永久 FileChanged**（document.rs:120、docStore.ts:84-86）——DELETE 失败不应使 save 整体失败，改 best-effort 记日志。
- **W2. 恢复后 mtime 基线丢失**（docStore.ts:61-69、document.rs:100-104）——restoreSession 置 meta=null，save 传 expectedMtime=undefined 跳过冲突检测，恢复的旧内容盲写覆盖磁盘新版本。恢复文件文档先 doc_open 拿 mtime 基线。
- **W3. FileChanged 冲突无解决路径**（docStore.ts:84、App.tsx:175）——错误横幅常驻，每次保存必败，UI 无"重新加载/强制覆盖"按钮，用户保留编辑则永远存不上。加冲突解决 UI + forceOverwrite 参数。
- **W4. index_file_content 的 DELETE+INSERT 无事务 + 新文件 FK 缺失被静默跳过索引**（index.rs:70-90、vault.rs:115-124、db_schema.sql）——中途失败留半索引且 scan 的 mtime+size 短路该文件永不修复；files 表无行的新文件 INSERT tags 违反 FK，`let _ =` 吞掉 → 搜索有、标签无。包事务 + 先 upsert files 行 + 索引错误记日志。
- **W5. reindex 先清空再重建、无事务，中途失败清空全索引**（index.rs:96-103）——且 DELETE files 级联删 recent/snapshots/attachments。整体包事务或改 upsert+对账。
- **W6. scan 只增不删，外部删除的文件永久残留索引**（vault.rs:52-84）——搜索仍返回已删文件。scan 结束对不在本次集合的行 DELETE。
- **W7. session_restore 把一切查询错误映射为 NotFound**（document.rs:184）——只对 QueryReturnedNoRows 映射，其他上抛。
- **W8. save 的 temp-rename 失败残留 `.{name}.ruach-tmp` 孤儿文件，且固定名会覆盖同名用户文件**（document.rs:116）——失败 best-effort remove_file；tmp 名加 pid 后缀；启动清理残留。

### 安全加固
- **W9. CSP 为 null**（tauri.conf.json:21）——配置最小 CSP（注意 dev HMR 需放行 ws://localhost:1421）。
- **W10. symlink/junction 逃逸**（document.rs:52、vault.rs:70）——合法 rel_path + vault 内 junction 即可读写 vault 外；canonicalize 前缀校验统一兜底。
- **W11. comrak 不净化 URL scheme**（render.rs:23、export.rs:66-71）——`[x](javascript:...)` 原样进导出 HTML，浏览器打开导出文件点击即执行。渲染后对 href/src 做 scheme 白名单。
- **W12. attach_paste 无输入大小限制**（attachment.rs:47）——数 GB data URL 可 OOM/填盘。解码前限长（64MB）。
- **W13. tauri-plugin-opener 注册但从未使用**（lib.rs:14、capabilities、package.json）——多余攻击面（webview 可任意开浏览器 URL）。确认不用则删四处。
- **W14. 同步命令在主线程执行，重命令阻塞 UI**（db.rs:22-24 注释与 Tauri 2 实际行为不符）——vault_scan/index_reindex/render_markdown/doc_save 全同步，大库卡顿数秒。重命令改 async + spawn_blocking，修正注释。

### 多窗口
- **W15. capability 只覆盖 `["main"]`，动态创建的 `editor-*` 窗口无任何权限**（capabilities/default.json:5、window.rs:12-18）——新窗口内 `listen("doc:changed")` 与 dialog 调用被 ACL 拒绝、静默失败 → 跨窗口同步与"打开 Vault"在次窗口失效。加 capability `"windows": ["main", "editor-*"]`。
- **W16. 新窗口不继承 vault 上下文**（window.rs:20-24、App.tsx:178）——`?doc=` 打开成功但 noVault 空态遮挡文档。window_create 带上 `?vault=<path>&doc=<path>`，App 启动先 openVault 再 openDoc。

### 前端状态
- **W17. 外部内容替换触发 onDocChange→setContent 把 dirty 误置 true（"打开即脏"）→ 双窗口 doc:changed 自激保存循环**（useCodeMirror.ts:66,85-94、docStore.setContent）——每次打开 1.5s 后必多一次写盘；两窗口同开一文档时约每 1.5s 重载+写盘+光标跳变无限循环。setContent 内容相同 no-op。
- **W18. 外部替换无 dirty 守卫，异步飞行期间输入被整档替换吞掉**（useCodeMirror.ts:88）——点树→IPC 在飞→打字→openDoc resolve 后替换整档吞字。替换前检查 dirty 则跳过。
- **W19. theme hydrate 与用户操作竞态**（App.tsx:50-58）——configLoad 返回前改设置会被覆盖；persist 会把未 hydrate 字段以默认值写回磁盘。hydrate 加 userInteracted 守卫或 await 完成。
- **W20. 预览渲染无乱序防护**（preview/index.tsx:133-148）——旧响应覆盖新结果 + spinner 提前熄灭。token/seq 比对。
- **W21. inlineAttachments 每次渲染无缓存重读全部附件**（preview/index.tsx:14-32）——分屏下每次键入 MB 级 IPC + base64 重建。模块级 Map 缓存，换文档/换 vault 清空。
- **W22. srcDoc 的 useMemo 缺依赖**（preview/index.tsx:150）——themeCss() 读 CSS 变量但 memo 只依赖 [body, theme]，改行距/页宽/字体后预览样式陈旧。订阅 fontPreset/lineHeight/pageWidth。
- **W23. 命令面板搜索无乱序防护**（command/index.tsx:152-165）——与 W20 同法修复。
- **W24. 恢复游标链路断裂**（docStore.ts:81、RecoveryBanner.tsx）——flush 不传 cursor、restore 丢弃 cursor、editorStore.cursor 死数据。透传或删除。
- **W25. 图片粘贴失败静默吞掉**（editor/index.tsx:36）——catch 空。setStatus 提示。
- **W26. RecoveryBanner restore 无 catch**（RecoveryBanner.tsx:30）——unhandled rejection + finally 无条件 discard。加 catch，成功后才 discard。
- **W27. 新建草稿无落盘出口，困在恢复区死循环**（docStore.ts:80-82）——无 save-as 流程。加 doc_save_as 命令或"保存为文件"按钮。
- **W28. 导出 PDF 实际不可用**（export.rs:57、preview/index.tsx:165）——后端 NotImplemented + 沙箱 iframe（sandbox=""）的 print() 被 HTML 规范禁止。实现后端打印管线，或前端无沙箱隐藏 iframe 承载打印；roadmap 如实标注。

### 可访问性
- **W29. 命令面板无焦点圈闭、无 dialog 语义、关闭不还原焦点**（command/index.tsx:193）——Tab 逃出面板、关闭后焦点落 body。focus trap + role="dialog" + 还原焦点。
- **W30. palette listbox 未与输入框绑定**（command/index.tsx:205）——缺 combobox/aria-controls/aria-activedescendant/option id。补全。
- **W31. 文件树仅 Tab 逐项遍历，目录按钮缺 aria-expanded**（tree/index.tsx:117）——按 WAI-ARIA tree 模式改造。
- **W32. segmented 按钮组不暴露选中态**（App.tsx:138）——加 aria-pressed 或 radiogroup。
- **W33. Button 的 active 属性是死代码**（Button.tsx:13）——`.btn--active` 无 CSS 规则也无 aria-pressed，"设置"开关无视觉反馈。补样式 + aria-pressed。
- **W34. 动态内容无 aria-live**（App.tsx:162,175、RecoveryBanner）——status-banner 加 role="status"、error-banner 加 role="alert"。
- **W35. palette__input 的 outline:none 抵消全局 :focus-visible**（App.css:437）——删除或改保留可见指示的写法。
- **W36. TabStrip 用 role="tab" 挂在不可聚焦 span 上**（tabs/index.tsx:10）——伪造语义；P6 实现前先去掉 role。
- **W37. --ink-faint 三主题下对比度均 < 4.5:1**（tokens.css:41 等）——warm 2.05:1 / cold 2.29:1 / night 4.27:1，用于占位符与 11px 提示文字。加深至达标。

### 配置/清理
- **W38. 锁持有期间执行阻塞文件 IO**（commands.rs:39-44）——save 闭包内含 fs::write、scan 闭包含全目录递归读，期间 search/session 等全排队。文件 IO 移出锁外。
- **W39. Mutex expect("poisoned") 共 5 处**（commands.rs:27,39,52,53、db.rs:45）——锁毒化后全应用连锁 panic。改 unwrap_or_else(into_inner) 或返回错误。
- **W40. 多处吞错无日志，全项目无日志设施**（document.rs:53、commands.rs:65,83）——open 把一切错误映射 NotFound；索引失败静默。引入 tracing 或至少 eprintln。
- **W41. urlencode 未编码 `+`**（window.rs:35）——新窗口打开 `a+b.md` 变 `a b.md`。补 `%2B`。
- **W42. ConfigService::save 非原子写**（config.rs:67）——崩溃留截断 JSON。temp+rename；load 失败回退默认值。
- **W43. db 版本过新用 InvalidColumnName 表达**（db.rs:54-56）——错误信息误导。加 SchemaVersion 变体。

### 性能
- **W44. 编辑器每次键入全量 doc.toString() 进 store**（useCodeMirror.ts:65）——O(n) 拷贝 × React 重渲染链。store 同步节流，save 从 view 直接读。
- **W45. onContainerRef 内联 ref 回调每次渲染重建，ResizeObserver 累积且从不 disconnect**（tree/index.tsx:88）——每次滚动新建 observer。useCallback + 卸载清理。
- **W46. buildTree 用 flat.find 循环内查目录，O(D×F) 二次方**（tree/index.tsx:34）——1000 目录 × 1 万文件 = 千万次比较。先建 Map 索引。
- **W47. FileTree ResizeObserver 从不 disconnect**（tree/index.tsx:93）——同 W45 合并处理。
- **W48. useCodeMirror 每次渲染返回新对象**（useCodeMirror.ts:96）——paste 监听每次键入重挂载。useCallback 稳定。
- **W49. editorStore.cursor 死数据**（editorStore.ts:4）——每次键入无效写。删除或接线上游。

---

## Nit 清单（合并）

- validate_rel_path 测试补 "C:foo"、含 `..` 合法名用例
- 重复代码：mtime 计算 4 处、stem 提取 4 处、测试临时目录 6 份拷贝 → 抽 services::util
- 测试缺口：snapshot 服务、迁移路径（v1→v2）、命令层、集成测试
- snapshot.rs 三个 NotImplemented 无测试（实现时补）
- `@codemirror/language` 声明未使用（package.json:15）
- `@ts-expect-error` 压制 process（vite.config.ts:4）——tsconfig.node.json 加 types:["node"]，build 改 `tsc -b`
- Cargo.toml 占位符（description="A Tauri App"、authors）与插件版本约束风格不一
- README/CLAUDE.md 测试现状描述三处不一致（CLAUDE.md 仍写 "No test suite configured yet"）
- recent 表零读写无说明——roadmap 遗留清单补记
- index:updated 事件从未 emit（commands.rs:138）——补发或规格注明预留
- doc:opened 无前端监听
- LIKE 兜底未转义 %/_（search.rs:56）；SearchHit.score 注释失实
- db_schema.sql 头注释 "user_version 1" 过时（实际 v2）
- error.rs 错误文案英文直出、code 零消费——建 code→中文文案映射表（规格 §六 未实现）
- 编辑器与预览字距不一致（预览 0.02em，编辑器无）
- error-banner 硬编码颜色 #7d3b2e/#f5e8e0（App.css:629）
- immersion-exit 0.5 透明度对比度 2.1:1
- tree__empty 样式不存在
- preview code 圆角 3px 未走 --radius-sm
- Ctrl+E 未处理 e.repeat（App.tsx:70）
- 编辑器容器 aria-label 无效（无 role div）——用 EditorView.contentAttributes
- preview 与编辑器 letter-spacing 不一致

---

## 修复优先级建议

**P0（数据丢失 / 安全 / 视觉全错）**：C1 主题、C2 attach/export 校验、C3 侧信道、C4 mtime 毫秒、C5 切换/关闭 flush、C6 save 清 dirty 竞态、C7 路径校验组件化
**P1（功能失效）**：W15 capability 多窗口、W16 新窗口 vault、W17 自激循环、W18 吞字、W2 restore 基线、W3 冲突 UI、W28 PDF、W9 CSP、W27 save-as、W4/W5/W6 索引事务与对账、W19 hydrate 竞态
**P2（性能 / a11y / 清理）**：W20-W26 前端细节、W29-W37 a11y、W38-W49 性能与健壮性、全部 Nits

## 验证记录

以下关键结论已人工抽查确认（非仅代理声明）：
- tokens.css 选择器 `:root[data-theme=...]` × App.tsx div 挂载 data-theme，无 dataset.theme 写入 ✓
- document.rs mtime 全部 `as_secs()` ✓
- sessionFlush 仅草稿路径调用，无 beforeunload/CloseRequested ✓
- validate_rel_path 仅 open/save 两处调用 ✓
- capabilities `"windows": ["main"]` ✓
