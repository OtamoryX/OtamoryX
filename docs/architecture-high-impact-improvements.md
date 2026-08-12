# OtamoryX 高价值架构改进建议

## 目标

这份文档只记录改完以后收益非常明显的架构问题，不收录低价值的代码整理、命名调整或局部重构建议。

评估标准：

- 会明显提升系统稳定性、演进效率或跨端一致性
- 问题已经进入真实运行路径，而不是停留在规划代码
- 值得投入中到高成本做一次成体系改造

## 与当前路线图的关系

这份文档不覆盖当前主路线图。

当前仍以 [docs/roadmap.md](/home/sober/OtamoryX/docs/roadmap.md) 中的插件运行时闭环和后续 AI 业务能力为主；本文档只定义：

- 主线功能完成后的高价值架构改造 backlog
- 或者在不打断当前主线前提下，可以先做设计与局部铺垫的事项

因此，本文中的“优先级”应理解为：

- 架构改造之间的优先级
- 不是对 P1 发布前问题的覆盖排序

## 结论摘要

当前主线完成后，最值得投入的改造仍然集中在 3 个方向：

1. 收敛后端装配根与持久化入口
2. 建立统一的后台作业运行时
3. 冻结 API 契约，并收口前端请求/会话平台层

但这 3 项不应该直接并行硬推，建议采用一个明确的前置阶段：

### Phase 0：先做最小收敛

先完成以下最小地基动作，再进入后续大改造：

- 冻结后端未来只保留一个真实运行时 `AppContext` 作为目标真相源
- 明确仓库内哪些 `AppState`/`Services`/`ProcessingPipeline`/`Storage` 是保留、删除还是冻结
- 明确数据库支持的架构基线：哪些是“当前保证可运行”，哪些是“保留中的能力声明”
- 前端统一认证恢复时机与请求入口，避免继续新增页面层兼容代码

如果 Phase 0 不先完成，后续不论是 job runtime 还是 API 契约改造，都很容易变成“在现有混乱上再叠一层”。

## 推荐顺序

### 路线 A：优先稳定性

1. Phase 0：装配根与请求平台层最小收敛
2. 统一后台作业运行时
3. 收敛后端装配根与持久化入口的剩余改造
4. 冻结 API 契约，并完成前端生成化接入

### 路线 B：优先开发效率

1. Phase 0：装配根与请求平台层最小收敛
2. 冻结 API 契约，并收口前端请求/会话平台层
3. 收敛后端装配根与持久化入口的剩余改造
4. 统一后台作业运行时

说明：

- 路线 A 适合当前更担心重复扫描、重复处理、后台任务不可见的问题
- 路线 B 适合前端仍在快速迭代，希望尽快减少字段兼容和页面层协议修复成本

## 0. Phase 0：先做最小收敛

### 为什么先做

当前最危险的不是“缺某个新基础设施”，而是系统缺少单一真相源：

- 后端运行时仍然是 `Pool<Sqlite>` 加多个 `Extension` 拼出来的
- 仓库里同时残留未接入主路径的 `AppState`/`Services` 原型
- 前端认证恢复、路由守卫、请求拦截并不共享一个统一入口

这会让任何后续大改造都面临同一个风险：

- 新方案不是替代旧方案，而是成为第三套方案

### 这一阶段只做什么

- 明确后端未来唯一运行时的目标形态，而不是先把所有 handler 一次性迁过去
- 前端收敛为一个真实请求入口和一个真实会话恢复入口
- 盘点并定性未接入主路径的原型结构
- 明确数据库支持的内部实现基线与外部能力表述

### 明确不做什么

- 不在这一阶段直接引入完整 repository 抽象
- 不在这一阶段直接上完整 job runtime
- 不在这一阶段直接要求所有 API 自动生成
- 不要求在这一阶段完成所有 handler 对 `Pool<Sqlite>` 依赖的迁移

这一阶段的目标不是“把终局一次做完”，而是先把后续改造的落脚点变成单一、可控、可迭代的结构。

## 1. 收敛后端装配根与持久化入口

### 问题

后端目前存在“有架构意图，但真实运行态仍然是基础设施直穿业务”的情况。

- `backend/src/main.rs` 定义并装配运行时
- `backend/src/lib.rs` 里又残留另一套未接入主路径的 `AppState`
- 路由实际注入的是 `Pool<Sqlite>` 加多个 `Extension`
- 数据库层声明支持 `Sqlite` 和 `Postgres`
- 启动流程又立即把数据库能力压回 SQLite 专用 helper
- 大量 handler/service 仍直接依赖 `Pool<Sqlite>` 和 `sqlx`

这意味着：

- 运行时只有一个真实方言，但代码里保留了半完成的多数据库叙事
- 依赖注入没有单一入口
- 持久化边界没有收口，导致基础设施耦合继续向 handler 扩散
- 如果直接做 job runtime、插件 host、测试替身，注入路径会继续分叉

### 证据

- [backend/src/main.rs](/home/sober/OtamoryX/backend/src/main.rs#L19)
- [backend/src/main.rs](/home/sober/OtamoryX/backend/src/main.rs#L55)
- [backend/src/main.rs](/home/sober/OtamoryX/backend/src/main.rs#L125)
- [backend/src/lib.rs](/home/sober/OtamoryX/backend/src/lib.rs#L15)
- [backend/src/database/mod.rs](/home/sober/OtamoryX/backend/src/database/mod.rs#L31)
- [backend/src/handlers/settings.rs](/home/sober/OtamoryX/backend/src/handlers/settings.rs#L53)
- [backend/src/services/processing_pipeline.rs](/home/sober/OtamoryX/backend/src/services/processing_pipeline.rs#L10)

### 建议

不要直接在“SQLite only”和“完整 repository/storage adapter”之间二选一跳跃推进，而是分两步：

1. 先收敛到一个具体可运行的 `Sqlite AppContext`
2. 再决定是否继续保留跨数据库抽象

第一步必须做到：

- handler 不再直接以 `Pool<Sqlite>` 作为长期公共接口
- 持久化访问先收敛到少数应用服务/持久化入口
- 清理或定性 `lib.rs` 中未接入运行路径的 `AppState`/`Services`
- 清理或定性 `ProcessingPipeline`/`Storage` 这类未正式接入主路径的抽象雏形
- 先按“SQLite 是当前唯一保证可运行的实现基线”设计装配根

第二步再决定：

1. 明确当前只支持 SQLite，删除假性的多数据库抽象
2. 如果确实要保留多数据库能力，再逐步引入 repository/storage adapter

这里要特别区分两件事：

- “内部实现先按 SQLite 基线收敛”是架构动作
- “是否正式撤回 PostgreSQL 对外支持”是产品/迁移决策

如果选择前者而不是后者，路线图和对外文档也必须同步说明：

- 当前代码现实中，SQLite 是唯一保证可运行的路径
- PostgreSQL 若继续保留，应视为待重新打通和重新验收的能力，而不是默认可靠能力

### 收益

- 让后续 job runtime、插件 host、缓存、权限等横切能力有统一注入点
- 降低“再叠一层基础设施”的结构性风险
- 让测试替身和系统集成测试更可控
- 能明确哪些能力是真支持，哪些只是保留中的设计意图

### 成本与风险

- 成本中到高
- 会影响大量 handler 和 service 的构造方式
- 但如果不先做，后续几项高价值改造都会缺少稳定落点

## 2. 建立统一的后台作业运行时

### 问题

扫描、重扫、异步插件执行、AI 分析、预处理等后台能力目前缺少统一的 durable job 运行时，而是分散在多个地方直接 `tokio::spawn` 或各自管理任务语义。

当前症状：

- 启动时扫描直接后台起任务
- 设置变更后重扫直接后台起任务
- 手动扫描直接后台起任务
- 目录扫描内部再派生并发任务
- 插件事件总线是内存态 fan-out
- 插件调度器当前会触发全部任务，且尚未做真实 cron 判定
- 插件执行本身又有独立超时、abort、防 panic 语义

这会导致：

- durable job 没有统一的并发预算、幂等键和结果跟踪
- 重启后没有恢复语义
- 文件抖动、重复点击、路径切换时容易重复处理
- 插件相关后台能力正在形成第二套异步运行模型

### 证据

- [backend/src/main.rs](/home/sober/OtamoryX/backend/src/main.rs#L94)
- [backend/src/handlers/settings.rs](/home/sober/OtamoryX/backend/src/handlers/settings.rs#L107)
- [backend/src/handlers/settings.rs](/home/sober/OtamoryX/backend/src/handlers/settings.rs#L264)
- [backend/src/services/archive_processing_service.rs](/home/sober/OtamoryX/backend/src/services/archive_processing_service.rs#L148)
- [backend/src/services/file_monitor_service.rs](/home/sober/OtamoryX/backend/src/services/file_monitor_service.rs#L105)
- [backend/src/services/plugin_event_bus.rs](/home/sober/OtamoryX/backend/src/services/plugin_event_bus.rs#L152)
- [backend/src/services/plugin_scheduler.rs](/home/sober/OtamoryX/backend/src/services/plugin_scheduler.rs#L147)
- [backend/src/services/plugin_executor.rs](/home/sober/OtamoryX/backend/src/services/plugin_executor.rs#L138)

### 关键边界

这里必须先把“统一什么”说清楚。

统一后台作业运行时，应该优先统一的是 durable jobs，而不是把所有异步行为都硬塞进一个系统。

建议按下面 3 类边界划分：

1. `Durable jobs`
   - 可重试、可持久化、可恢复、需要状态跟踪
   - 例如：全库扫描、路径变更后的重扫、AI 分析、缩略图/封面预处理、批量插件执行
2. `Ephemeral tasks`
   - 生命周期短，不需要持久化恢复
   - 例如：单次请求内的受控插件执行
3. `Control loops / event fan-out`
   - 常驻 watcher、进程内事件分发、调度 tick
   - 它们负责发现事件和入队，不直接等价于 job 本身

文件监控本身不应该被建模成一条持久化 job；它应该是“发现变化并投递 job”的控制循环。

插件事件总线也不应该直接被替换成 jobs 表；它更适合作为进程内分发层，再由需要 durable 语义的消费者显式入队。

### 建议

引入统一的 durable job runtime，而不是继续增加分散的后台任务入口。

建议最少包含：

- `jobs` 持久化表
- worker coordinator
- 幂等键
- single-flight 控制
- 状态机：`pending/running/succeeded/failed/cancelled`
- 重试、超时、限流、并发预算
- 基础可观测性：任务来源、耗时、失败原因

建议第一批纳入统一运行时的任务类型：

- 启动扫描
- 手动全库扫描
- 路径切换后的重扫
- AI 分析
- 缩略图/封面预处理
- 需要异步结果跟踪的批量插件执行

建议暂时不直接纳入为 job 本体的内容：

- 文件监控 watcher 自身
- 进程内插件事件总线
- 单次请求内同步返回的 one-shot 插件调用

### 收益

- 这是最直接影响运行稳定性的系统级改造之一
- 能显著降低重复处理、IO 打满、任务丢失和运维不可见的问题
- 为插件系统和 AI 异步能力提供统一的 durable 基础设施

### 成本与风险

- 成本高
- 会调整多个现有服务之间的职责边界
- 如果在 Phase 0 之前直接推进，极易把范围做爆

## 3. 冻结 API 契约，并收口前端请求/会话平台层

### 问题

前后端问题已经不只是“字段名漂移”，而是前端协议层、请求层、认证恢复层都缺少单一真相源。

当前现象：

- 前端 client 统一 `baseURL` 为 `/api/v1`
- `getHealth()` 仍请求 `/health`，这更像现存错误而不只是漂移症状
- 前端存在 `id/plugin_id`、`archiveId/archive_id`、`pluginId/plugin_id` 等兼容逻辑
- 页面有的走 `utils/api.ts`，有的直接写 `fetch`
- 页面层会自行注入 token，绕过统一 401 处理
- 路由鉴权、session 恢复和 API 错误处理边界并不统一

这说明：

- API 协议已经不是单一真相源
- 页面层正在承担协议修复和认证恢复工作
- 即使引入 OpenAPI/codegen，如果不先收口请求/会话平台层，生成代码也会与手写兼容层长期并存

### 证据

- [frontend/src/utils/api.ts](/home/sober/OtamoryX/frontend/src/utils/api.ts#L45)
- [frontend/src/utils/api.ts](/home/sober/OtamoryX/frontend/src/utils/api.ts#L76)
- [frontend/src/utils/api.ts](/home/sober/OtamoryX/frontend/src/utils/api.ts#L98)
- [frontend/src/utils/api.ts](/home/sober/OtamoryX/frontend/src/utils/api.ts#L132)
- [backend/src/main.rs](/home/sober/OtamoryX/backend/src/main.rs#L128)
- [frontend/src/router/index.ts](/home/sober/OtamoryX/frontend/src/router/index.ts#L47)
- [frontend/src/App.vue](/home/sober/OtamoryX/frontend/src/App.vue#L117)
- [frontend/src/stores/auth.ts](/home/sober/OtamoryX/frontend/src/stores/auth.ts#L17)
- [frontend/src/views/ReaderView.vue](/home/sober/OtamoryX/frontend/src/views/ReaderView.vue#L745)
- [frontend/src/views/SettingsView.vue](/home/sober/OtamoryX/frontend/src/views/SettingsView.vue#L848)
- [frontend/src/views/SettingsView.vue](/home/sober/OtamoryX/frontend/src/views/SettingsView.vue#L1413)
- [frontend/src/views/SettingsView.vue](/home/sober/OtamoryX/frontend/src/views/SettingsView.vue#L1527)

### 建议

这一项需要拆成两层同时推进：

1. 冻结后端 DTO / OpenAPI 契约
2. 收口前端请求与会话平台层

具体建议：

- 从 Rust 侧导出 OpenAPI 或等价契约
- 前端类型和 client 自动生成
- 页面层禁止直接手写协议兼容
- 页面层禁止直接 `fetch` 后端业务接口
- 只保留一条数据访问通道
- 明确 auth state 的单一 owner；拦截器、路由守卫、session bootstrap 只能经由这一 owner 改写登录态
- 统一认证恢复时机，避免路由守卫与 session bootstrap 互相打架
- 统一 401 处理、错误映射、缓存失效和 token 注入
- 在生成化 client 切入后，逐步删除字段双写和运行时 fallback

建议优先落地的最小动作：

1. 明确 auth state 的单一 owner，并统一认证恢复入口和路由守卫顺序
2. 禁止新增页面层直连 `fetch`
3. 优先选“高频且兼容/fallback 最重”的接口接入生成化 client
4. 删除对应页面的兼容层和 fallback
5. 顺手修正 `getHealth()` 这类已知错误调用

### 收益

- 明显减少跨端字段漂移
- 减少登录态恢复、401 跳转、请求注入不一致造成的前端回归
- 降低页面重复写请求逻辑的成本
- 减少 query key、缓存更新和错误处理不一致带来的问题

### 成本与风险

- 成本中等
- 需要梳理现有 API 命名、返回结构和前端认证流转
- 但如果继续拖后，页面层兼容逻辑会持续扩张

## 不建议现在优先处理的事项

以下问题目前不建议排到前面：

- 单纯按文件大小拆文件
- 组件命名或目录轻量整理
- 未进入真实运行路径的规划代码美化
- 不改变依赖方向的局部 service 抽取
- 在单一装配根尚未建立前，直接补一层“看起来很完整”的抽象容器

这些事情可以做，但不属于当前最值得投入的架构改进。

## 建议落地方式

### 先做设计冻结，再做实现切片

对这 3 项改造，建议都先产出一页级别的实施设计，再按最小可验证切片推进，而不是一次性大重构。

推荐切法：

1. Phase 0
   - 冻结 `AppContext` 目标形态并清理原型噪音
   - 前端统一 session bootstrap + request gateway
2. 后端装配根
   - 先统一 SQLite 运行时
   - 再决定是否保留多数据库抽象
3. Job runtime
   - 先接扫描类 durable jobs
   - 再接 AI 和批量插件执行
4. API 契约
   - 先接 1 组高频接口
   - 再逐步删兼容层和手写 fallback

## 一句话判断

如果只允许先做一件事：

- 偏线上稳定性：先做 Phase 0，然后进入“统一后台作业运行时”
- 偏开发效率：先做 Phase 0，然后进入“API 契约 + 前端请求/会话平台层收口”
