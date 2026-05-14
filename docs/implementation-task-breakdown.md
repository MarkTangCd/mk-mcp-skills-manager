# AgentHub Local 实施任务拆分

## 使用方式

这个文档用于把 AgentHub Local 从空项目目录逐步实现出来。每个任务块都可以单独交给 coding agent 执行。

建议执行方式：

1. 严格按阶段顺序执行。
2. 每次只给 coding agent 一个任务块。
3. 每个任务完成后要求 agent 输出变更文件、验证命令、已知问题。
4. 进入下一个阶段前，先确认当前阶段所有验收标准通过。
5. 不允许 coding agent 跳过备份、测试、lint、类型检查和文档更新。

## 全局实现约束

- 目标是 macOS 本地桌面应用。
- 技术栈使用 Tauri 2 + React + TypeScript + Vite + Rust + SQLite。
- 前端代码使用 2 spaces indentation。
- 前端变量使用 camelCase。
- 常量使用 UPPER_SNAKE_CASE。
- 优先使用 async/await。
- 代码注释使用英文。
- 默认不执行 MCP server、Pi extension、package 或用户项目代码。
- 所有配置写入必须走 `scan -> plan -> diff -> backup -> apply -> rescan -> record`。
- Agent Adapter 不直接写文件，只负责 scan、buildChangePlan、validateAppliedChange、runDoctor。
- ChangeService 统一负责备份、原子写入、恢复和变更记录。
- SQLite 是索引和历史记录，不是配置真实来源。
- 文件系统中的 agent 配置才是真实来源。

## 推荐项目目录

目标代码仓库从空目录初始化后，建议目录如下：

```text
agenthub-local/
  src/
    app/
    components/
    features/
    lib/
    routes/
    styles/
    types/
  src-tauri/
    src/
      adapters/
      commands/
      db/
      domain/
      fixtures/
      security/
      services/
      utils/
    migrations/
    capabilities/
  fixtures/
    agents/
      claude-code/
      codex/
      opencode/
      pi/
  docs/
    architecture/
    qa/
  package.json
  README.md
```

说明：

- `src/` 保存 React 前端。
- `src-tauri/src/domain/` 保存领域模型和核心流程。
- `src-tauri/src/adapters/` 保存 agent adapter。
- `src-tauri/src/services/` 保存 scan、change、backup、doctor、prompt 等服务。
- `fixtures/agents/` 保存 agent 配置样本。
- `docs/` 保存实现过程中的开发文档和 QA 清单。

## 交给 Coding Agent 的通用提示词模板

```text
你正在实现 AgentHub Local，一个 macOS 本地桌面应用。

请只完成当前任务块，不要提前实现后续阶段。

全局约束：
- 技术栈：Tauri 2 + React + TypeScript + Vite + Rust + SQLite。
- 前端使用 2 spaces indentation、camelCase 变量、UPPER_SNAKE_CASE 常量。
- 代码注释使用英文。
- 不执行 MCP server、Pi extension、package 或用户项目代码。
- Adapter 不直接写文件；配置写入必须通过 ChangeService。
- 每次修改后运行可用的 lint、typecheck、test 或 build。

当前任务：
[粘贴本任务块]

完成后请输出：
1. 修改的文件列表。
2. 运行过的验证命令及结果。
3. 未完成事项或风险。
```

## Phase 0: 项目初始化与开发基线

目标：从空目录创建可运行、可测试、可持续迭代的 Tauri + React 项目骨架。

### T00-01 初始化 Tauri + React + TypeScript 项目

依赖：无。

任务：

- 在空项目目录初始化 Tauri 2 + React + TypeScript + Vite 应用。
- 配置 package manager，优先使用 `pnpm`。
- 设置基础 `package.json` scripts：
  - `dev`
  - `build`
  - `typecheck`
  - `lint`
  - `format`
  - `test`
  - `tauri:dev`
  - `tauri:build`
- 初始化 Rust 侧 `src-tauri`。
- 确认 macOS 下可以启动空应用窗口。

产出：

- 可运行的桌面应用骨架。
- `README.md`，说明本地开发命令。

验收标准：

- `pnpm install` 成功。
- `pnpm typecheck` 成功。
- `pnpm build` 成功。
- `pnpm tauri:dev` 可以启动应用。
- README 中包含开发环境、启动命令和项目结构说明。

建议给 coding agent 的提示：

```text
请从空目录初始化 AgentHub Local 项目，使用 Tauri 2 + React + TypeScript + Vite。只完成项目骨架和基础开发命令，不实现业务功能。
```

### T00-02 配置代码质量工具

依赖：T00-01。

任务：

- 配置 ESLint。
- 配置 Prettier。
- 配置 TypeScript strict 模式。
- 配置 Rust formatter 和 clippy 使用说明。
- 增加基础 `.editorconfig`。
- 增加 `.gitignore`。
- 增加 `docs/qa/development-checklist.md`。

产出：

- 统一格式化和静态检查配置。
- 开发检查清单。

验收标准：

- `pnpm lint` 成功。
- `pnpm format` 可运行。
- `pnpm typecheck` 成功。
- `cargo fmt --check` 成功。
- `cargo clippy` 无 error。

建议给 coding agent 的提示：

```text
请为现有 Tauri + React 项目配置 ESLint、Prettier、TypeScript strict、Rust fmt/clippy 和基础开发检查清单。不要实现业务页面。
```

### T00-03 建立基础 UI 框架

依赖：T00-02。

任务：

- 建立应用主布局。
- 增加左侧导航。
- 增加顶部状态区域。
- 创建空路由页面：
  - Dashboard
  - Projects
  - MCP Servers
  - Skills
  - Sub-agents
  - Pi Resources
  - Prompts
  - Doctor
  - Backups
  - Settings
- 增加基础主题变量。
- UI 风格保持开发者工具导向，密集、清晰、少装饰。

产出：

- 可导航的空壳应用。
- 基础路由和布局组件。

验收标准：

- 每个页面都能通过侧边栏进入。
- 页面刷新后路由可恢复。
- 无明显布局溢出。
- `pnpm typecheck` 和 `pnpm build` 成功。

建议给 coding agent 的提示：

```text
请实现 AgentHub Local 的基础 UI 框架和空路由页面。重点是开发者工具式布局，不要做营销页，不要实现业务数据。
```

## Phase 1: 本地核心与数据库基础

目标：建立 Rust 领域模型、SQLite migration、本地数据目录和 Tauri command 基础。

### T01-01 定义领域模型

依赖：T00-03。

任务：

- 在 Rust 侧定义核心类型：
  - Agent
  - Project
  - ConfigScope
  - McpServer
  - Skill
  - SubAgent
  - PiResource
  - PromptTemplate
  - ResourceBinding
  - ScanSnapshot
  - DoctorIssue
  - ChangeSet
  - Backup
- 定义枚举：
  - AgentKind
  - ScopeType
  - ResourceType
  - HealthStatus
  - IssueSeverity
  - ChangeStatus
- 定义前端共享类型导出策略。

产出：

- Rust domain model。
- TypeScript 对应类型或自动生成策略。

验收标准：

- Rust 类型可序列化 / 反序列化。
- TypeScript 侧能安全消费 Tauri command 返回值。
- `cargo test` 成功。
- `pnpm typecheck` 成功。

建议给 coding agent 的提示：

```text
请实现 AgentHub Local 的核心领域模型。只定义类型、枚举和序列化结构，不实现扫描和写入逻辑。
```

### T01-02 初始化 SQLite 与 Migration

依赖：T01-01。

任务：

- 集成 SQLite。
- 建立 migrations：
  - agents
  - projects
  - config_scopes
  - resources
  - resource_bindings
  - scan_snapshots
  - doctor_issues
  - change_sets
  - backups
  - prompt_templates
  - settings
- 实现数据库初始化。
- 增加 migration 测试。

产出：

- 本地 SQLite 初始化能力。
- 初始 schema。

验收标准：

- 首次启动自动创建数据库。
- 重复启动不会破坏已有数据库。
- migration 测试通过。
- schema 字段能覆盖技术方案中的核心模型。

建议给 coding agent 的提示：

```text
请为 AgentHub Local 实现 SQLite 初始化和第一版 migration。不要实现具体业务页面，只保证数据库可初始化和测试通过。
```

### T01-03 实现 App Data 目录管理

依赖：T01-02。

任务：

- 实现 app data 目录解析。
- 启动时创建：
  - `library/skills`
  - `library/sub-agents`
  - `library/prompts`
  - `library/mcp-templates`
  - `backups`
  - `logs`
  - `cache/scans`
- 定义路径安全校验工具。
- 禁止未授权路径写入。

产出：

- AppDataService。
- PathGuard。

验收标准：

- 启动后目录结构存在。
- 路径 traversal 测试通过。
- symlink 写入策略有测试或明确后续任务。
- `cargo test` 成功。

建议给 coding agent 的提示：

```text
请实现 AgentHub Local 的 app data 目录管理和路径安全校验。重点是创建本地数据目录和防止路径 traversal。
```

### T01-04 建立 Tauri Command 基础

依赖：T01-03。

任务：

- 建立 command 分层：
  - app commands
  - projects commands
  - agents commands
  - doctor commands
  - changes commands
  - backups commands
  - prompts commands
- 统一错误模型：
  - code
  - message
  - target
  - recoverable
  - details
- 前端实现基础 API client。
- Dashboard 页面调用 `app.getDashboard()` 返回空状态。

产出：

- Rust command 框架。
- 前端 API client。
- Dashboard 空状态数据接入。

验收标准：

- Dashboard 能显示来自 Rust command 的真实数据。
- 错误能被前端统一展示。
- `pnpm build` 成功。
- `cargo test` 成功。

建议给 coding agent 的提示：

```text
请建立 Tauri command 基础、统一错误模型和前端 API client，并让 Dashboard 读取一个真实的空状态 command。
```

## Phase 2: Fixture 与只读扫描骨架

目标：在不碰真实用户配置的情况下，先用 fixture 建立 Adapter、扫描和 Matrix 的基础能力。

### T02-01 建立 Agent Adapter Trait

依赖：T01-04。

任务：

- 定义 Rust 侧 AgentAdapter trait：
  - kind
  - detectInstallation
  - locateGlobalConfig
  - locateProjectConfig
  - scan
  - buildChangePlan
  - validateAppliedChange
  - runDoctor
- 实现 AdapterRegistry。
- 增加 mock adapter 用于测试。

产出：

- Adapter trait。
- AdapterRegistry。
- MockAdapter。

验收标准：

- 可以注册多个 adapter。
- 可以按 AgentKind 获取 adapter。
- mock adapter 测试通过。

建议给 coding agent 的提示：

```text
请实现 Agent Adapter trait、AdapterRegistry 和 MockAdapter。不要实现真实 Claude/Codex/opencode/Pi 逻辑。
```

### T02-02 建立配置 Fixture 样本目录

依赖：T02-01。

任务：

- 建立 `fixtures/agents/`。
- 为四类 agent 建立最小样本目录：
  - claude-code
  - codex
  - opencode
  - pi
- 每类至少包含：
  - empty config
  - valid global config
  - valid project config
  - duplicate mcp config
  - invalid config
- 增加 fixture README，说明样本是假数据，不能包含真实 secret。

产出：

- Fixture 样本文件。
- Fixture README。

验收标准：

- fixture 文件不包含真实 token、key、个人路径。
- 测试可以加载 fixture。
- README 说明样本用途。

建议给 coding agent 的提示：

```text
请建立 AgentHub Local 的 agent 配置 fixtures。全部使用假数据，不包含真实路径和 secret。不要实现真实扫描逻辑。
```

### T02-03 实现 ScanService 骨架

依赖：T02-02。

任务：

- 实现 ScanService。
- 支持扫描项目下所有已注册 adapters。
- 扫描失败不阻断其他 adapter。
- 将扫描 summary 写入 scan_snapshots。
- 支持读取最新扫描快照。

产出：

- ScanService。
- scan_snapshots 写入和查询。

验收标准：

- mock adapter 扫描结果能写入数据库。
- 一个 adapter 失败时其他 adapter 仍可完成。
- 有单元测试覆盖成功和失败路径。

建议给 coding agent 的提示：

```text
请实现 ScanService 骨架，使用 MockAdapter 验证扫描、错误隔离和 scan_snapshots 写入。不要实现真实 agent adapter。
```

### T02-04 实现 Projects 基础管理

依赖：T02-03。

任务：

- 实现添加项目。
- 实现项目列表。
- 实现项目详情读取。
- 添加项目时校验路径存在且是目录。
- 前端 Projects 页面接入真实 command。

产出：

- ProjectService。
- Projects 页面基础列表。
- Add Project 表单。

验收标准：

- 可以添加本地项目路径。
- 重复路径不会创建重复项目。
- 无效路径显示错误。
- 项目列表从 SQLite 读取。
- `pnpm build` 和 `cargo test` 成功。

建议给 coding agent 的提示：

```text
请实现 Projects 基础管理，包括添加项目、列表、详情和前端接入。不要实现 Matrix 和真实 agent 扫描。
```

### T02-05 实现 Project Scan 入口

依赖：T02-04。

任务：

- 在 Project 详情页增加 Rescan 操作。
- 调用 ScanService。
- 展示最近扫描时间、扫描状态、错误摘要。
- 当前阶段使用 MockAdapter 或 fixture adapter。

产出：

- Project scan command。
- Project 详情扫描入口。

验收标准：

- 点击 Rescan 后能产生 ScanSnapshot。
- 页面能展示扫描结果摘要。
- 失败 adapter 不影响整体页面。

建议给 coding agent 的提示：

```text
请实现项目详情页的 Rescan 入口，调用 ScanService 并展示扫描摘要。当前可以继续使用 MockAdapter，不要接真实配置路径。
```

## Phase 3: 只读 Agent Adapters

目标：实现四类 agent 的只读检测和扫描，不做任何写入。

### T03-01 实现 opencode 只读 Adapter

依赖：T02-05。

任务：

- 基于 fixture 实现 opencode 配置解析。
- 识别 MCP servers。
- 识别 enabled 状态。
- 输出规范化 McpServer resources。
- 增加 duplicate MCP 测试。
- 暂不写真实用户配置。

产出：

- OpencodeAdapter read-only。
- Parser tests。

验收标准：

- valid fixture 可解析。
- empty fixture 返回空资源。
- invalid fixture 返回可恢复错误。
- duplicate fixture 生成 doctor issue 或 scan warning。

建议给 coding agent 的提示：

```text
请实现 opencode 的只读 Adapter，只基于 fixtures 解析 MCP servers 和 enabled 状态。不要写入任何配置。
```

### T03-02 实现 Claude Code 只读 Adapter

依赖：T03-01。

任务：

- 基于 fixture 实现 Claude Code 配置解析。
- 扫描 MCP。
- 扫描 Skills。
- 扫描 Sub-agents。
- 标记资源来源 scope。

产出：

- ClaudeCodeAdapter read-only。
- Parser tests。

验收标准：

- fixture 中 MCP / Skills / Sub-agents 都能规范化。
- invalid config 有清晰错误。
- 不访问真实 `~` 路径，除非显式测试注入。

建议给 coding agent 的提示：

```text
请实现 Claude Code 的只读 Adapter，先只支持 fixture 解析 MCP、Skills、Sub-agents。不要访问真实用户目录。
```

### T03-03 实现 Codex 只读 Adapter

依赖：T03-02。

任务：

- 基于 fixture 实现 Codex 配置解析。
- 扫描 MCP。
- 扫描 Skills。
- 扫描 Custom Agents。
- 标记全局 / 项目来源。

产出：

- CodexAdapter read-only。
- Parser tests。

验收标准：

- fixture 解析成功。
- Custom Agents 能映射为 SubAgent resource。
- Skills 能映射为 Skill resource。
- 不做写入。

建议给 coding agent 的提示：

```text
请实现 Codex 的只读 Adapter，先只支持 fixture 解析 MCP、Skills、Custom Agents。不要实现写入。
```

### T03-04 实现 Pi 只读 Adapter

依赖：T03-03。

任务：

- 基于 fixture 实现 Pi settings 解析。
- 扫描 Skills。
- 扫描 Prompt Templates。
- 扫描 Extensions。
- 扫描 Packages。
- 扫描 Themes。
- 对 extension 标记 trusted / untrusted 初始状态。

产出：

- PiAdapter read-only。
- PiResource parser tests。

验收标准：

- settings fixture 可解析。
- resource path 不存在时生成 warning。
- extension 不执行，只读取 metadata。
- Pi 不输出 SubAgent resource。

建议给 coding agent 的提示：

```text
请实现 Pi 的只读 Adapter，基于 fixture 解析 settings、skills、prompt templates、extensions、packages、themes。不要执行 extension。
```

### T03-05 实现 Agent 检测空安全模式

依赖：T03-04。

任务：

- 为四个 adapter 实现 installation detection 的安全版本。
- 优先使用配置路径存在性和固定命令检测。
- 如果命令不可用，返回 not installed，不报 fatal error。
- 前端 Agents / Dashboard 展示检测结果。

产出：

- Agent detection service。
- Dashboard agent cards。

验收标准：

- 未安装 agent 时应用正常运行。
- 检测失败显示 warning，不崩溃。
- Dashboard 展示四个 agent 状态。

建议给 coding agent 的提示：

```text
请实现四个 agent 的安全安装检测和 Dashboard 展示。检测失败不能导致应用崩溃，不要执行任何 agent 扩展能力。
```

## Phase 4: Matrix 与只读管理界面

目标：把扫描结果变成用户可用的 Dashboard、Project Matrix 和资源列表。

### T04-01 实现资源索引写入

依赖：T03-05。

任务：

- ScanService 将 adapter 输出写入 `resources` 和 `resource_bindings`。
- 保留来源路径、agent、scope、project。
- 支持重新扫描后更新旧资源状态。

产出：

- ResourceIndexer。
- resource upsert 逻辑。

验收标准：

- 重复扫描不会创建无意义重复记录。
- resource binding 能表达 agent + project + scope。
- 测试覆盖新增、更新、删除或 missing 状态。

建议给 coding agent 的提示：

```text
请实现扫描结果的资源索引写入，把 Adapter 输出持久化到 resources 和 resource_bindings，并支持重复扫描更新。
```

### T04-02 实现 Project Matrix API

依赖：T04-01。

任务：

- 实现 `projects.getMatrix(projectId)`。
- 输出：
  - MCP Matrix
  - Skills Matrix
  - Sub-agent Matrix
  - Pi Resource Summary
- Matrix cell 包含 enabled、disabled、missing、unknown 状态。

产出：

- Matrix query service。
- Matrix API response type。

验收标准：

- 有 fixture 测试覆盖多 agent 聚合。
- Project Matrix 能区分全局和项目级来源。
- Pi Resources 不进入 Sub-agent Matrix。

建议给 coding agent 的提示：

```text
请实现 Project Matrix API，基于 resources 和 bindings 聚合 MCP、Skills、Sub-agents 和 Pi Resource Summary。
```

### T04-03 实现 Project Matrix UI

依赖：T04-02。

任务：

- 在 Project 详情页展示：
  - MCP Matrix
  - Skills Matrix
  - Sub-agent Matrix
  - Pi Resource Summary
- 支持基础过滤。
- 点击 cell 展示来源详情。

产出：

- MatrixTable 组件。
- Project Detail Matrix UI。

验收标准：

- 多列情况下横向滚动可用。
- cell 状态颜色清晰。
- 来源详情展示 config path、scope、agent。
- `pnpm build` 成功。

建议给 coding agent 的提示：

```text
请实现 Project Detail 的 Matrix UI，展示 MCP、Skills、Sub-agents 和 Pi Resource Summary，并支持点击查看来源详情。
```

### T04-04 实现资源列表页只读版

依赖：T04-03。

任务：

- MCP Servers 页面展示 resources 中 MCP。
- Skills 页面展示 Skill。
- Sub-agents 页面展示 SubAgent。
- Pi Resources 页面展示 Pi resources。
- 支持搜索、agent filter、project filter。

产出：

- MCP / Skills / Sub-agents / Pi Resources 列表页。

验收标准：

- 列表数据来自 SQLite。
- 搜索和过滤可用。
- 空状态清晰。
- 不包含写入按钮或写入按钮禁用并标明后续阶段。

建议给 coding agent 的提示：

```text
请实现 MCP、Skills、Sub-agents、Pi Resources 的只读列表页，数据来自 SQLite resources，不实现编辑和写入。
```

## Phase 5: Doctor 只读检查

目标：实现 PRD 中 P0 风险检查，但不做一键修复。

### T05-01 实现 Doctor 规则引擎

依赖：T04-04。

任务：

- 实现 DoctorService。
- 支持 issue severity：
  - info
  - warning
  - critical
- 支持 category：
  - agent
  - mcp
  - skill
  - sub-agent
  - pi
- 写入 doctor_issues。

产出：

- DoctorService。
- DoctorIssue persistence。

验收标准：

- 可针对 project 或 all 运行 doctor。
- issue 可去重。
- issue 可被下一次扫描刷新。

建议给 coding agent 的提示：

```text
请实现 DoctorService 规则引擎和 doctor_issues 持久化。先建立框架和 issue 生命周期，不做复杂规则。
```

### T05-02 实现 MCP Doctor Rules

依赖：T05-01。

任务：

- 检查重复 MCP。
- 检查缺失 env。
- 检查疑似明文 secret。
- 检查危险 command。
- 检查 disabled 但被项目引用。

产出：

- MCP doctor rules。
- 测试 fixture。

验收标准：

- 每条规则有测试。
- secret 检测不把 secret 原文写入数据库。
- dangerous command 规则有明确匹配说明。

建议给 coding agent 的提示：

```text
请实现 MCP Doctor Rules，包括重复、缺失 env、疑似 secret、危险 command、disabled 引用。注意不要把 secret 原文写入 DB 或日志。
```

### T05-03 实现 Skill / Sub-agent / Pi Doctor Rules

依赖：T05-02。

任务：

- Skill：
  - 缺少说明
  - 缺少入口文件
  - 路径失效
  - 未被任何项目使用
- Sub-agent：
  - 同名冲突
  - 绑定 MCP 不存在
  - 绑定 Skill 不存在
  - 权限过大
- Pi：
  - resource path 不存在
  - package 重复
  - extension 未信任
  - project settings 覆盖 global

产出：

- Skill / Sub-agent / Pi doctor rules。
- 对应测试。

验收标准：

- 每类至少 3 个规则有测试。
- Pi extension 不执行。
- 权限过大规则先使用静态启发式判断。

建议给 coding agent 的提示：

```text
请实现 Skill、Sub-agent、Pi 的 Doctor Rules。所有规则只读检查，不做自动修复，不执行 Pi extension。
```

### T05-04 实现 Doctor UI

依赖：T05-03。

任务：

- Doctor 页面展示 issue 列表。
- 支持 severity filter。
- 支持 category filter。
- 支持按项目过滤。
- Dashboard 展示风险摘要。

产出：

- Doctor 页面。
- Dashboard issue summary。

验收标准：

- critical issue 明显突出。
- issue 展示目标资源、来源路径、建议动作。
- Dashboard 和 Doctor 页面数据一致。

建议给 coding agent 的提示：

```text
请实现 Doctor 页面和 Dashboard 风险摘要，只展示问题和建议，不实现一键修复。
```

## Phase 6: Change Plan、Diff、Backup、Restore 基础

目标：建立写入安全链路，但先只对 fixture 或 AgentHub 自有文件写入。

### T06-01 定义 ChangeIntent 与 ChangePlan

依赖：T05-04。

任务：

- 定义 ChangeIntent。
- 定义 ChangePlan。
- 定义 ChangeOperation。
- 定义 FilePatch。
- 定义 DiffSummary。
- 设计变更状态：
  - draft
  - previewed
  - confirmed
  - applied
  - applied_with_warning
  - failed
  - restored

产出：

- Change domain types。
- change_sets 存储支持。

验收标准：

- ChangePlan 可序列化给前端。
- change_sets 可保存 draft 和 applied 状态。
- 测试覆盖状态转换。

建议给 coding agent 的提示：

```text
请定义 ChangeIntent、ChangePlan、ChangeOperation、FilePatch、DiffSummary 和状态转换。不要实现真实文件写入。
```

### T06-02 实现 Diff Preview

依赖：T06-01。

任务：

- 实现文本 diff 生成。
- ChangePlan 包含目标文件、变更摘要、风险。
- 前端实现 Diff Preview 页面或侧边面板。
- 危险变更显示二次确认提示。

产出：

- DiffService。
- DiffPreview UI。

验收标准：

- 能展示 before / after diff。
- 大文件有截断策略。
- 用户未确认不能 apply。

建议给 coding agent 的提示：

```text
请实现 ChangePlan 的 diff 生成和前端 Diff Preview。只做预览，不写入文件。
```

### T06-03 实现 BackupService

依赖：T06-02。

任务：

- 写入前复制目标文件到 backups。
- 创建 `manifest.json`。
- 记录原路径、hash、size、changeSetId、createdAt。
- 支持列出 backups。

产出：

- BackupService。
- backups table 写入。

验收标准：

- 对不存在目标文件有明确处理策略。
- manifest 可用于恢复。
- hash 测试通过。

建议给 coding agent 的提示：

```text
请实现 BackupService，支持写入前文件快照、manifest.json 和 backups 列表。不要实现 restore。
```

### T06-04 实现 ChangeService Apply

依赖：T06-03。

任务：

- 实现统一写入入口。
- Apply 前必须检查：
  - plan 已确认
  - 路径允许写入
  - backup 已创建或可创建
- 使用 temp file + atomic rename。
- 写入后 hash 校验。
- 写入后调用 adapter validateAppliedChange。
- 记录 change status。

产出：

- ChangeService apply。
- 安全写入测试。

验收标准：

- 未确认 plan 不能写。
- 未授权路径不能写。
- 写入失败保留 backup。
- 测试只写 fixture 或临时目录。

建议给 coding agent 的提示：

```text
请实现 ChangeService apply，包含确认检查、路径校验、backup、temp file、atomic rename、hash 校验和状态记录。测试只能写临时目录或 fixture copy。
```

### T06-05 实现 Restore

依赖：T06-04。

任务：

- 支持恢复单个备份文件。
- 支持恢复一个 ChangeSet 涉及的所有文件。
- 恢复后重新扫描受影响 scope。
- 前端 Backups 页面支持 restore 操作。

产出：

- Restore service。
- Backups UI。

验收标准：

- 恢复后文件 hash 与备份一致。
- 恢复操作有二次确认。
- 恢复后 change status 更新为 restored。

建议给 coding agent 的提示：

```text
请实现备份恢复能力和 Backups 页面 restore 操作。恢复必须二次确认，恢复后更新 ChangeSet 状态并触发 rescan。
```

## Phase 7: MCP 写入能力

目标：优先实现最核心的 MCP Manager 写入闭环。

### T07-01 实现 MCP ChangeIntent 表单

依赖：T06-05。

任务：

- MCP 新增表单。
- MCP 编辑表单。
- 启用 / 禁用操作。
- 删除操作。
- 选择目标 agent。
- 选择目标 scope：
  - global
  - project

产出：

- MCP change form。
- MCP change intent builder。

验收标准：

- 表单校验 command、args、url、envRefs。
- env 只允许变量名，不允许明文值。
- 点击提交后只生成 ChangePlan，不直接写入。

建议给 coding agent 的提示：

```text
请实现 MCP 新增、编辑、启用/禁用、删除的表单和 ChangeIntent builder。提交后只生成 ChangePlan，不直接写入。
```

### T07-02 实现 opencode MCP ChangePlan

依赖：T07-01。

任务：

- opencode adapter 支持 MCP 新增、编辑、删除、启用、禁用的 buildChangePlan。
- 基于结构化解析生成目标配置。
- 输出 diff。
- 不直接写文件。

产出：

- OpencodeAdapter MCP buildChangePlan。
- fixture tests。

验收标准：

- 新增 MCP diff 正确。
- 禁用 MCP diff 正确。
- 删除 MCP diff 正确。
- invalid config 返回可恢复错误。

建议给 coding agent 的提示：

```text
请为 opencode adapter 实现 MCP buildChangePlan，支持新增、编辑、删除、启用、禁用。只返回 ChangePlan，不直接写文件。
```

### T07-03 实现 Claude / Codex MCP ChangePlan

依赖：T07-02。

任务：

- Claude Code adapter 支持 MCP ChangePlan。
- Codex adapter 支持 MCP ChangePlan。
- 处理全局 / 项目 scope。
- 增加同名冲突检测。

产出：

- ClaudeCodeAdapter MCP buildChangePlan。
- CodexAdapter MCP buildChangePlan。
- tests。

验收标准：

- 每个 adapter 至少覆盖新增、编辑、启用、禁用。
- 同名冲突输出 warning。
- 不保存 secret 原文。

建议给 coding agent 的提示：

```text
请为 Claude Code 和 Codex adapters 实现 MCP buildChangePlan，覆盖新增、编辑、启用、禁用和同名冲突。不要直接写文件。
```

### T07-04 接通 MCP Apply Flow

依赖：T07-03。

任务：

- MCP 表单生成 ChangePlan。
- Diff Preview 展示。
- 用户确认后 ChangeService apply。
- Apply 后 rescan。
- Matrix 和 MCP 列表刷新。

产出：

- MCP 写入闭环。

验收标准：

- 只能在确认后写入。
- 写入前创建 backup。
- 写入后 Matrix 反映新状态。
- Restore 可恢复写入前状态。

建议给 coding agent 的提示：

```text
请接通 MCP Manager 的完整写入流程：表单 -> ChangePlan -> Diff Preview -> Confirm -> Backup -> Apply -> Rescan -> UI refresh。
```

## Phase 8: Skills 与 Sub-agents

目标：实现 AgentHub Library、Skill 引用 / 同步、Sub-agent 创建和启用。

### T08-01 实现 AgentHub Library 文件管理

依赖：T07-04。

任务：

- 管理 app data 下：
  - library/skills
  - library/sub-agents
  - library/prompts
  - library/mcp-templates
- 创建 LibraryService。
- 支持 slug 校验。
- 支持 metadata 文件。

产出：

- LibraryService。
- library metadata schema。

验收标准：

- slug 使用小写 kebab-case。
- 创建重复 slug 报错。
- Library 文件写入走 ChangeService 或受控内部写入策略。

建议给 coding agent 的提示：

```text
请实现 AgentHub Library 文件管理，包括 skills、sub-agents、prompts、mcp-templates 目录和 metadata schema。
```

### T08-02 实现 Skill Library MVP

依赖：T08-01。

任务：

- 创建 Skill metadata。
- 导入已有 Skill。
- 标签和搜索。
- Skill 列表和详情。
- 检查缺少说明、缺少入口文件。

产出：

- SkillService。
- Skills UI。

验收标准：

- 可以创建 Skill。
- 可以导入本地 Skill。
- 搜索可用。
- Doctor 能识别 Skill 基础问题。

建议给 coding agent 的提示：

```text
请实现 Skill Library MVP：创建 metadata、导入、标签、搜索、列表、详情和基础 Doctor 检查。
```

### T08-03 实现 Skill 启用到 Agent

依赖：T08-02。

任务：

- Claude Code / Codex 支持 Skill 同步 ChangePlan。
- Pi 支持通过 resource path 引用 AgentHub Library。
- 记录 ResourceBinding。
- 不强制支持 opencode 写入。

产出：

- Skill enable flow。
- Adapter-specific Skill ChangePlan。

验收标准：

- Claude / Codex 同步需要 diff preview。
- Pi resource path 修改需要 backup。
- opencode 显示 unsupported 状态。

建议给 coding agent 的提示：

```text
请实现 Skill 启用到 Claude Code、Codex 和 Pi 的流程。Claude/Codex 使用同步，Pi 优先使用 resource path 引用。opencode 暂不写入。
```

### T08-04 实现 Sub-agent Library MVP

依赖：T08-03。

任务：

- 创建 Sub-agent。
- 编辑基础 metadata。
- 绑定 MCP 和 Skills。
- 从内置模板创建。
- Sub-agent 列表和详情。

产出：

- SubAgentService。
- Sub-agents UI。
- 内置模板。

验收标准：

- 可以创建 Code Reviewer / Debugger 等模板。
- 可以绑定已有 MCP 和 Skill。
- Pi 不出现在 Sub-agent 适用 agent 中。

建议给 coding agent 的提示：

```text
请实现 Sub-agent Library MVP，包括创建、编辑、绑定 MCP/Skills、模板创建、列表和详情。Pi 不支持 Sub-agent。
```

### T08-05 实现 Sub-agent 启用到 Claude / Codex

依赖：T08-04。

任务：

- Claude Code adapter 支持 Sub-agent ChangePlan。
- Codex adapter 支持 Custom Agent ChangePlan。
- 支持 global / project scope。
- 检测同名冲突。

产出：

- Sub-agent enable flow。
- Claude / Codex adapter support。

验收标准：

- 启用前有 diff preview。
- 同名冲突给 warning 或阻断。
- 写入后 Matrix 更新。
- Restore 可用。

建议给 coding agent 的提示：

```text
请实现 Sub-agent 启用到 Claude Code 和 Codex 的流程，支持 global/project scope、同名冲突检测、Diff Preview、Backup、Apply、Rescan。
```

## Phase 9: Pi Resources 与 Prompts

目标：完成 Pi 专属资源管理和通用 Prompt Copy Library。

### T09-01 完善 Pi Resources UI

依赖：T08-05。

任务：

- Pi Overview。
- Pi Settings。
- Pi Skills。
- Pi Prompt Templates。
- Pi Extensions。
- Pi Packages。
- Pi Themes。
- 展示来源、路径、状态和风险。

产出：

- Pi Resources 页面组。

验收标准：

- Pi extension 不执行。
- 不存在路径明确 warning。
- project settings 覆盖 global settings 有提示。

建议给 coding agent 的提示：

```text
请完善 Pi Resources 页面组，展示 settings、skills、prompt templates、extensions、packages、themes 的来源、路径、状态和风险。不要执行 extension。
```

### T09-02 实现 Pi Settings ChangePlan

依赖：T09-01。

任务：

- 支持编辑 Pi resource paths。
- 支持引用 AgentHub Skill Library。
- 支持启用 / 禁用 skill commands 等安全设置。
- 所有变更走 ChangeService。

产出：

- Pi settings edit flow。

验收标准：

- 修改前有 diff preview。
- 修改前有 backup。
- 修改后 rescan。
- Restore 可用。

建议给 coding agent 的提示：

```text
请实现 Pi Settings 的 ChangePlan 和写入流程，重点支持 resource paths 和 AgentHub Skill Library 引用。所有写入走 ChangeService。
```

### T09-03 实现 Prompt Library

依赖：T09-02。

任务：

- 创建 Prompt。
- 编辑 Prompt。
- 分类、标签、收藏。
- 搜索。
- 变量识别。
- 变量填写。
- 最终 prompt 预览。
- 复制到剪贴板。

产出：

- PromptService。
- Prompts UI。

验收标准：

- 变量格式使用 `{{variableName}}`。
- 缺失变量有提示。
- 复制内容是渲染后的最终 prompt。
- 不写入任何 agent 配置。

建议给 coding agent 的提示：

```text
请实现通用 Prompt Library，包括创建、编辑、分类、搜索、变量填写、预览和复制。不要写入任何 agent 配置。
```

## Phase 10: MVP 打磨与发布准备

目标：让应用具备可自用的稳定 MVP 质量。

### T10-01 性能和错误处理打磨

依赖：T09-03。

任务：

- 扫描过程增加 loading 和进度。
- 大量资源列表增加分页或虚拟滚动。
- 统一错误 toast / inline error。
- Rust 日志落盘并做 secret redaction。

产出：

- 更稳定的错误处理。
- 性能优化。

验收标准：

- 1000 条 resources 列表不卡顿。
- 扫描失败可恢复。
- 日志不包含 secret 原文。

建议给 coding agent 的提示：

```text
请对 AgentHub Local 做 MVP 打磨：扫描进度、列表性能、统一错误展示、日志和 secret redaction。
```

### T10-02 完整端到端 QA

依赖：T10-01。

任务：

- 建立 QA checklist。
- 覆盖：
  - 首次启动
  - 添加项目
  - 扫描
  - Matrix
  - Doctor
  - MCP 写入
  - Backup
  - Restore
  - Skill 启用
  - Sub-agent 启用
  - Pi resource path
  - Prompt 复制
- 修复 QA 发现的 P0/P1 问题。

产出：

- `docs/qa/mvp-qa-checklist.md`。
- QA 修复。

验收标准：

- QA checklist 全部通过或记录明确例外。
- 没有已知会破坏用户配置的 bug。

建议给 coding agent 的提示：

```text
请建立并执行 MVP QA checklist，覆盖核心流程。修复 P0/P1 问题，低优先级问题记录为已知问题。
```

### T10-03 macOS 打包

依赖：T10-02。

任务：

- 配置 Tauri macOS bundle。
- 设置 app name、icon、identifier。
- 生成本地安装包。
- README 增加安装和运行说明。

产出：

- macOS app build。
- 打包说明。

验收标准：

- `pnpm tauri:build` 成功。
- 本地打开 app 成功。
- 首次启动能创建 app data。
- 没有意外请求过宽权限。

建议给 coding agent 的提示：

```text
请配置 AgentHub Local 的 macOS Tauri 打包，设置 app metadata、icon 占位和 README 安装说明，确保本地 build 成功。
```

## 推荐执行顺序总览

| Phase    | 重点                      | 可交付结果                   |
| -------- | ------------------------- | ---------------------------- |
| Phase 0  | 项目初始化                | 可启动空应用                 |
| Phase 1  | 本地核心                  | DB、目录、command 基础       |
| Phase 2  | Fixture 和扫描骨架        | 可用 mock 扫描项目           |
| Phase 3  | 只读 adapters             | 能读取四类 agent fixture     |
| Phase 4  | Matrix 和列表             | 可视化项目资源状态           |
| Phase 5  | Doctor                    | 只读风险检查                 |
| Phase 6  | Change / Backup / Restore | 安全写入基础设施             |
| Phase 7  | MCP 写入                  | 核心写入闭环                 |
| Phase 8  | Skills / Sub-agents       | Library 和启用               |
| Phase 9  | Pi / Prompts              | Pi 专属资源和 Prompt Library |
| Phase 10 | MVP 打磨                  | 可自用打包版本               |

## MVP 完成定义

MVP 可以认为完成，当满足：

- 应用可以在 macOS 本地启动和打包。
- 可以添加项目并扫描。
- Dashboard 能展示四类 agent 状态。
- Project Matrix 能展示 MCP、Skills、Sub-agents、Pi Resources。
- MCP Manager 支持至少 opencode、Claude Code、Codex 的写入闭环。
- Sub-agent Manager 支持 Claude Code 和 Codex 的创建与启用。
- Pi Resources 能查看并管理 resource paths。
- Prompt Library 能创建、变量渲染和复制。
- Doctor 能发现 P0 风险。
- Backup / Restore 对所有写入路径可用。
- 没有明文 secret 写入 DB 或日志。

## 第一批最小可执行任务

如果只想先启动开发，建议先按以下 5 个任务发给 coding agent：

1. T00-01 初始化 Tauri + React + TypeScript 项目。
2. T00-02 配置代码质量工具。
3. T00-03 建立基础 UI 框架。
4. T01-01 定义领域模型。
5. T01-02 初始化 SQLite 与 Migration。

这 5 个任务完成后，项目从空目录进入可持续开发状态。
