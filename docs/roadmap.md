# OtamoryX 开发路线图

**最后更新**: 2026-03-08
**当前状态**: Phase 6 已完成；P1 发布前阻断项收尾中（已复核收敛 P1-009/P1-010）；Phase 7 插件系统已进入实施（P7-8 首批交付已验收通过，下一阶段聚焦 P7-3/P7-4 真实运行时闭环）

---

## 已完成里程碑（摘要）

### Phase 1-4: 核心功能 (v0.1.0 - v0.4.0) ✅

后端基础设施（Axum + SQLite/PostgreSQL）、认证与用户管理、压缩包处理（CBZ/CBR/CB7）、
搜索与分页、标签/分类系统、阅读进度追踪、多用户权限管理、RBAC 角色控制。

### Phase 5: 高级功能 (v0.5.0 - v0.5.5) ✅

系统配置管理、批量操作、健康监控、PostgreSQL 双数据库支持、目录浏览器、OPDS 1.2 协议。

### Phase 6: 前端 UI/UX 现代化 (v0.6.0) ✅

LANraragi 风格界面重构、移动端优先响应式设计、暗色/亮色主题、随机轮播、
标签自动补全搜索、LibraryView 重写（1128行→469行）。

---

## 安全与性能评审（2026-03-01，2026-03-08 复核）

> 由架构师、性能专家、安全专家、代码审查专家四角色联合评审发现。
> 历史问题明细文档已移除，当前仅保留 Issue 编号与状态追踪。

### P0 — 严重生产故障（必须立即修复）

- 已修复（简略）：P0-001, P0-002, P0-003, P0-004, P0-005, P0-006。

### P1 — 高风险（发布前必须修复）

- 已修复（简略）：P1-003, P1-004, P1-005, P1-007, P1-009, P1-010, P1-012, P1-014。

**待修复明细**

| Issue | 问题 | 状态 |
|-------|------|------|
| P1-001 | 用户 API 在内存中加载 password_hash | 🔴 待修复 |
| P1-002 | 无暴力破解防护 | 🟡 部分完成（已有用户名限流，仍缺 IP 维度与审计闭环） |
| P1-006 | 路径权限中间件形同虚设 | 🔴 待修复 |
| P1-008 | 压缩包被重复解压两次 | 🟡 待复核 |
| P1-011 | 缓存写锁粒度过粗 | 🟡 待复核 |
| P1-013 | Handler 直接 SQL 绕过 Service 层 | 🔴 待修复 |
| P1-015 | 封面生成逻辑重复 120 行 | 🟡 部分完成 |
| P1-016 | thumbnail handler 157 行重复代码 | 🟡 部分完成 |
| P1-017 | update_user 动态 SQL 绑定顺序脆弱 | 🟡 部分完成 |

### 后续修复顺序（仅未完成项）

1. P1-001 + P1-002 + P1-006（安全与权限）
2. P1-008 + P1-011（性能与稳定性）
3. P1-013 + P1-015 + P1-016 + P1-017（架构与可维护性）

---

## 未来里程碑（未完成工作清单）

### 当前迭代：发布前阻断项（P1 收尾）

**安全与权限**
- P1-001：用户查询链路彻底移除 `password_hash` 读取，补充最小字段查询与回归测试。
- P1-002：登录防暴力破解（IP + 用户维度限流、失败计数、冷却时间、审计日志）。
- P1-006：修复路径权限中间件，统一权限检查入口，补齐越权访问测试。

**性能与稳定性**
- P1-008：消除压缩包重复解压，统一提取流程并增加缓存命中验证。
- P1-011：优化缓存锁竞争（降低写锁粒度或引入分片/并发 Map 方案）。

**已收敛（2026-03-08 复核）**
- P1-009：`categories`、`batch_progress` 已改批量查询。
- P1-010：认证链路已改为 JWT 本地校验，显著减少认证 DB 查询。

**架构与可维护性**
- P1-013：清理 Handler 直连 SQL，回收至 Service/Repository 分层。
- P1-015：抽取封面生成公共逻辑，删除重复实现。
- P1-016：合并 thumbnail handler 重复逻辑，统一错误处理与鉴权路径。
- P1-017：重构 `update_user` 动态 SQL 组装，消除绑定顺序脆弱问题。

### Phase 7: 插件系统 (v1.1.0) — 按实现顺序细化

**目标边界（先统一）**
- v1 采用 `trusted plugin` 模式，权限声明用于审核/审批/审计，不承诺 OS 级强沙箱。
  - 设计依据：`docs/plugin-system-design.md` §1.3、§7.1、§7.3
- 当前插件系统无存量用户与线上依赖，Phase 7 不要求兼容旧插件实现（含旧 manifest/旧 ABI/旧库表），可直接重置并按新规范落地。

**已完成（保持简略）**
- 插件基础 CRUD（列表、安装、启停、配置、卸载）链路打通。
  - 设计依据：`docs/plugin-system-design.md` §9.1
- `plugin_id` 字段已贯通至 API/模型/执行记录，插件核心表结构已重置到 v1 基线（`plugins`、`plugin_executions`、`plugin_tags`）。
- `plugin.toml` v1 解析与基础校验已落地（含 `manifest_version` / `plugin_api_version` / `config_schema` 校验）。

**当前进度快照（2026-03-05）**
- P7-1（规范冻结与基线重置）：🟡 部分完成。已明确不兼容旧插件并按 v1 基线推进；`plugin_id` 与版本字段已落位。ABI 双向一致性校验尚未闭环到真实加载链路。
- P7-2（Manifest 与存储层）：🟢 基本完成。manifest 解析/校验、主库表结构、执行记录与关联索引已落地。
- P7-3（Runtime 核心）：🟡 框架已建。`PluginManager`/`PluginExecutor`、超时与 panic 防护已在位；动态库扫描加载、符号绑定、真实 FFI 调用仍为 TODO。
- P7-4（Host Callback 与权限网关）：🟡 部分完成。`OtamoryxHostApiV1`、权限网关与审计日志已实现；`db_query` 尚未接入共享数据库执行器。
- P7-5（API 层与系统集成）：🟡 部分完成。配置 schema、执行接口、执行历史接口已开放；内置插件已接入真实执行与结果落库，外部动态库插件仍以 `pending` 记录为主。
- P7-6（事件系统与调度）：🟡 原型完成。`PluginEventBus` 与 `PluginScheduler` 已有实现；cron 到期计算与业务触发链路尚未打通。
- P7-7（前端管理能力）：🟢 进展领先。插件类型筛选、权限展示、schema 驱动配置弹窗、执行记录视图、Reader one-shot 入口均已接入。
- P7-8（内置/官方首批插件）：🟢 首批交付完成。4 个内置插件已具备可执行逻辑并覆盖单元测试；2 个官方插件 manifest（`ehentai-metadata`、`nhentai-metadata`）已纳入启动引导幂等落库。
- P7-9（开发者体验与分发）：⚪ 尚未开始。
- 工程健康快照：backend `cargo check` 可通过；frontend 构建仍存在既有 TypeScript 报错，待单独收敛。

**P7-8 验收结果（2026-03-05）**
- ✅ `cargo fmt --all --check` 通过（已完成格式门禁收口）。
- ✅ `cargo check` 通过。
- ✅ `RUSTC_WRAPPER= cargo test --lib` 通过（25 passed / 0 failed）。
- ✅ 验收结论：P7-8（4 内置 + 2 官方）达到“可交付完成”状态。

**下一阶段执行计划（项目经理视角）**
1. P7-3 Runtime 真实链路闭环（动态库扫描/加载、符号绑定、`call_ffi_with_host_api` 落地）。
2. P7-4 Host Callback 收口（`db_query` 接入共享执行器、错误码与内存释放协议联调）。
3. P7-5 外部插件执行语义对齐（从 `pending` 过渡到真实执行结果回写与失败隔离）。
4. P7-6 事件与调度打通（cron 到期计算、archive 事件触发接入业务链路）。

**实施顺序（建议按 PR/Sprint 执行）**

1. **P7-1 规范冻结与基线重置（先做）**
- 冻结 `plugin.toml` v1 字段：`id/name`、`manifest_version`、`plugin_api_version`、`plugin_dependencies`、`config_schema`。
- 明确 ABI 校验规则：主程序加载时同时校验 manifest 与 `otamoryx_plugin_info` 的 `plugin_api_version`（仅面向新 v1 规范，不做旧版本兼容层）。
- 明确外部标识统一使用 `plugin_id`（API、DB、执行记录）。
- 设计依据：`docs/plugin-system-design.md` §4.2、§4.3、§6.3、§8.1、§9.1、§20.1

2. **P7-2 Manifest 与存储层收敛**
- 实现 `plugin.toml` 解析与验证器（包含 schema 基础校验和版本检查）。
- 直接重建插件表与执行表：`plugins`、`plugin_executions`、`plugin_tags`（统一 `plugin_id` 外键，不保留旧结构兼容迁移）。
- 建立 `PluginManifest` 持久化/反序列化流程。
- 设计依据：`docs/plugin-system-design.md` §4.2、§6.3、§8.1、§13 (Phase 1)

3. **P7-3 Runtime 核心（Manager/Executor）**
- 实现扫描、加载、卸载、启用、禁用完整生命周期。
- 实现 `PluginManager` + `PluginExecutor`，并采用函数指针缓存方案（避免 `Symbol` 生命周期陷阱）。
- 接入超时、panic 捕获、安全调用包装。
- 设计依据：`docs/plugin-system-design.md` §5、§6.2、§6.4、§7.3、§17.1-§17.3

4. **P7-4 Host Callback 与权限网关**
- 落地 `OtamoryxHostApiV1`：`http_request/db_query/fs_read/fs_write/free_string`。
- 在 callback 入口统一做域名/路径/表级权限检查与审计日志记录。
- SDK 错误码映射与内存释放协议对齐。
- 设计依据：`docs/plugin-system-design.md` §4.3、§7.1、§7.2、§15 附录 C、§22.1(问题4已收敛)

5. **P7-5 API 层完善与现有系统集成**
- 补齐插件执行、执行历史、配置 schema 查询接口。
- 与 `ProcessingPipeline`、`AppState`、动态路由转发集成（Endpoint 插件）。
- 加入执行冷却、批量执行错误隔离和恢复策略。
- 设计依据：`docs/plugin-system-design.md` §6.5、§9.1、§17.4、§18

6. **P7-6 事件系统与调度**
- 实现 `PluginEventBus` 与订阅声明解析。
- 打通 `archive_added/archive_updated/scheduled` 触发链路。
- Script 类型的 cron 调度落地。
- 设计依据：`docs/plugin-system-design.md` §16、§5.2、§13 (Phase 3)

7. **P7-7 前端管理能力补全**
- Plugins 管理页增强：类型筛选、执行记录入口、权限展示。
- 配置 UI 改为 schema 自动渲染（基于 `config_schema`）。
- Reader 详情页增加 one-shot 执行入口。
- 设计依据：`docs/plugin-system-design.md` §10.1、§10.2、§10.3、§14.3、§13 (Phase 4)

8. **P7-8 内置插件与官方插件首批落地**
- 先交付 P0 内置插件：`filename-parser`、`comicinfo-parser`、`date-added`、`tag-copier`。
- 再交付首批官方在线插件：`ehentai-metadata`、`nhentai-metadata`（按可信源安装流程）。
- 固化执行顺序与冲突处理策略。
- 设计依据：`docs/plugin-system-design.md` §21.3、§21.5、§19.4、§21.6

9. **P7-9 开发者体验与分发**
- 插件模板、开发模式、测试工具链补齐。
- 本地包安装规范完善，在线市场仅保留接口预留（不在 v1 实装）。
- 设计依据：`docs/plugin-system-design.md` §11.1、§11.3、§12、§13

**Phase 7 验收标准**
- 至少 2 个外部插件 + 4 个内置插件在同一实例稳定运行 7 天，无进程级崩溃。
- 插件执行链路具备可观测性（执行记录、错误分类、健康状态）。
- 管理端可完成安装、启停、配置、执行、查看历史、卸载全流程。
- 设计依据：`docs/plugin-system-design.md` §17、§18、§21

### Phase 8: AI 功能 (v1.2.0) — 基础设施部分完成

**已完成（保持简略）**：AI 设置管理与处理队列。

**待完成**
- 模型接入层：多提供商适配（本地/云端）、超时重试、成本统计。
- 内容分析流水线：封面/元数据/文本抽取任务编排与失败重试。
- 自动标签生成：可配置策略（阈值、黑白名单、语言映射）。
- 人工审核界面：批量确认/拒绝、差异对比、回滚能力。
- 质量评估：标签准确率抽样、延迟与成本监控看板。

### Phase 9: 桌面应用 (v1.3.0) 

- 建立 Tauri 工程骨架（窗口生命周期、配置加载、日志落盘）。
- 完成系统托盘能力（快速显示/隐藏、扫描状态、退出控制）。
- 打通桌面端与后端通信（本地 API 启停、端口冲突与健康检测）。
- 接入原生文件能力（文件选择器、目录授权、拖拽导入）。
- 构建跨平台打包流水线（Linux/macOS/Windows）与签名发布流程。

### Phase 10: 生产就绪 (v1.4.0)

- 测试体系：单元/集成/E2E 覆盖关键流程（认证、扫描、阅读、权限）。
- 性能基线：建立压测场景与 SLO（吞吐、P95 延迟、内存峰值）。
- 国际化 (i18n)：前后端文案抽离、多语言资源管理与回归检查。
- API 文档：OpenAPI 3.0 自动生成、鉴权示例与错误码规范。
- 发布工程：版本策略、迁移脚本、回滚预案、发布检查清单。
