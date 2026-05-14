# MKAgentHub Local 产品需求文档 PRD

## 1. 产品名称

### AgentHub Local

### ## 2. 产品定位

### AgentHub Local 是一个面向程序员的本地桌面应用，用于统一管理多个 Coding Agent 的 MCP、Skills、Sub-agents、Pi Resources 和 Prompt 模板。

### 目标是让用户不用在不同工具、不同项目、不同配置文件之间来回切换，就能清楚地看到：

### - 每个 Coding Agent 配置了哪些 MCP

### - 每个项目启用了哪些 Skills

### - Claude Code / Codex 下有哪些 Sub-agents

### - Pi coding agent 加载了哪些 skills、prompts、extensions、packages

### - 常用 Prompt 模板如何快速复用

### ## 3. 推荐产品平台

### ### 3.1 产品形态

### 推荐做成：

### **macOS Desktop App**

### 3.2 选择原因

用户主要使用 macOS，并且该产品需要管理本机上的 Coding Agent 配置、本地项目目录、本地 Skills 仓库和本地 Prompt 模板。

相比 Web App，桌面应用更适合处理本地文件、本地项目、本地 agent 配置和本地备份。

## 4. 产品背景

现在程序员在工作中会同时使用多个 Coding Agent，例如：

- Claude Code
- Codex
- opencode
- Pi coding agent

这些工具都支持不同形式的扩展能力。

Codex 支持 MCP、Skills 和 Subagents / Custom Agents，可以通过专门的 agent 执行并行探索、复杂任务拆解和定制化工作流。 [oai_citation:0‡OpenAI Developers](https://developers.openai.com/codex/subagents?utm_source=chatgpt.com)

opencode 支持通过配置文件管理 MCP servers，并且 MCP 配置中可以包含 enabled 状态。 [oai_citation:1‡OpenCode](https://opencode.ai/docs/mcp-servers/?utm_source=chatgpt.com)

Pi coding agent 的扩展体系更偏向 resources，包括 TypeScript extensions、skills、prompt templates、themes 和 packages。 [oai_citation:2‡Pi Dev](https://pi.dev/docs/latest?utm_source=chatgpt.com)

这些能力虽然强大，但目前存在明显管理痛点：

1. 不同 agent 的配置入口分散。
2. 全局配置和项目级配置容易混淆。
3. MCP server 经常重复配置，难以统一维护。
4. Skills 分散在不同 agent 或项目目录里。
5. Sub-agent 缺少统一管理入口。
6. Pi 的 skills、prompts、extensions、packages 和其他 agent 的 MCP / Sub-agent 体系不同，需要单独管理。
7. 常用 Prompt 模板缺少统一沉淀和快速复制入口。
8. 手动修改配置风险高，容易出错。
9. 不清楚某个项目到底启用了哪些 agent 能力。

## 5. 产品目标

### 5.1 核心目标

AgentHub Local 希望解决的问题是：

> 让程序员通过一个本地桌面应用，统一查看、管理和复用多个 Coding Agent 的 MCP、Skills、Sub-agents、Pi Resources 和 Prompt 模板。

### 5.2 具体目标

1. 统一管理多个 Coding Agent。
2. 统一管理 MCP servers。
3. 建立一个统一的 Skills 仓库。
4. 支持 Claude Code / Codex 的 Sub-agent 管理。
5. 针对 Pi coding agent 单独管理其 Resources。
6. 支持项目级配置视图。
7. 支持常用 Prompt 模板管理和复制。
8. 所有配置变更前都能让用户清楚知道会影响什么。
9. 降低手动编辑配置文件的心智负担。

## 6. 非目标

MVP 阶段不做以下内容：

1. 不做云同步。
2. 不做团队协作。
3. 不做在线 Marketplace。
4. 不做 Agent Chat UI。
5. 不替代 Claude Code、Codex、opencode、Pi。
6. 不自动运行 MCP 或 extensions。
7. 不托管 MCP server。
8. 不把 API Key 明文保存到产品中。
9. 不自动把通用 Prompt 模板写入项目文件。
10. 不做复杂的多 agent workflow 编排。

## 7. 目标用户

### 7.1 主要用户

重度使用 Coding Agent 的程序员。

### 7.2 用户特征

用户通常具备以下特点：

1. 同时使用多个 Coding Agent。
2. 经常在不同项目之间切换。
3. 会为不同项目配置不同 MCP。
4. 会沉淀自己的 Skills、Prompts、Sub-agents。
5. 希望减少重复配置。
6. 希望更清楚地知道每个项目启用了什么能力。
7. 希望配置修改更安全、可控、可回滚。

## 8. 核心产品概念

## 8.1 Agent

Agent 指一个 Coding Agent 工具。

MVP 重点支持：

1. Claude Code
2. Codex
3. opencode
4. Pi coding agent

## 8.2 MCP Server

MCP Server 是为 Coding Agent 提供外部工具能力的服务。

例如：

- GitHub
- Filesystem
- Playwright
- Context7
- Figma
- Browser Tools
- Database Tools

## 8.3 Skill

Skill 是一组可复用的任务能力说明，通常包含：

- 使用场景
- 工作流程
- 参考资料
- 脚本
- 模板

产品中会建立一个统一 Skill Library，让用户先在 AgentHub 中管理，再同步或引用到不同 agent。

## 8.4 Sub-agent

Sub-agent 是一个专门负责某类任务的专家 agent。

例如：

- Code Reviewer
- Debugger
- Security Auditor
- Frontend Architect
- Test Writer
- Refactor Planner
- Product Doc Reviewer

Claude Code 和 Codex 都适合做 Sub-agent 管理。Codex 官方也明确支持 Subagents 和 Custom Agents。 [oai_citation:3‡OpenAI Developers](https://developers.openai.com/codex/subagents?utm_source=chatgpt.com)

## 8.5 Pi Resources

Pi coding agent 的扩展方式和 Claude / Codex 不完全一样。

Pi 重点管理：

- Settings
- Skills
- Prompt Templates
- Extensions
- Packages
- Themes

Pi 官方文档说明，Pi 是一个 minimal terminal coding harness，核心保持轻量，通过 extensions、skills、prompt templates、themes 和 packages 扩展。 [oai_citation:4‡Pi Dev](https://pi.dev/docs/latest?utm_source=chatgpt.com)

## 8.6 Prompt Template

Prompt Template 是用户常用提示词模板。

AgentHub 中的通用 Prompt 模板只做：

- 管理
- 分类
- 搜索
- 变量填写
- 预览
- 复制

不自动写入项目文件。

## 9. 产品信息架构

AgentHub Local
├── Dashboard
├── Projects
├── MCP Servers
├── Skills
├── Sub-agents
├── Pi Resources
├── Prompts
├── Agents
├── Doctor
├── Backups
└── Settings

# 10. Dashboard 首页

# 10.1 页面目标

让用户快速了解当前本机 Coding Agent 的整体状态。

# 10.2 展示内容

1 已检测到的 Coding Agents。
2 每个 Agent 的安装状态。
3 每个 Agent 当前管理的 MCP 数量。
4 Skills 数量。
5 Sub-agents 数量。
6 Pi Resources 数量。
7 最近使用的项目。
8 最近修改的配置。
9 当前配置风险提醒。

⠀10.3 示例内容

### Detected Agents

### Claude Code

### - Installed

### - 12 MCP

### - 8 Skills

### - 5 Sub-agents

### Codex

### - Installed

### - 8 MCP

### - 6 Skills

### - 4 Custom Agents

### opencode

### - Installed

### - 5 MCP

### - 4 Skills

### Pi

### - Installed

### - 12 Skills

### - 6 Prompt Templates

### - 3 Extensions

### - 2 Packages

# 11. Projects 项目模块

# 11.1 页面目标

让用户以“项目”为中心查看当前项目启用了哪些 Agent 能力。
这是产品中非常重要的页面，因为用户真实工作流通常是围绕项目展开的。

# 11.2 核心问题

Project 页面需要回答：
这个项目里，Claude Code / Codex / opencode / Pi 分别启用了哪些 MCP、Skills、Sub-agents 和 Resources？

# 11.3 项目列表页

展示：
1 项目名称。
2 项目路径。
3 已识别的 Agents。
4 MCP 数量。
5 Skills 数量。
6 Sub-agents 数量。
7 Pi Resources 数量。
8 最近扫描时间。
9 健康状态。

⠀11.4 项目详情页
建议采用 Matrix 视图。

### MCP Matrix

### MCP Server Claude Code Codex opencode

### GitHub Enabled Enabled Disabled

### Filesystem Enabled Enabled Enabled

### Playwright Enabled Disabled Enabled

### Context7 Enabled Enabled Disabled

### Skills Matrix

### Skill Claude Code Codex opencode Pi

### EVM Wallet Debugging Enabled Enabled Disabled Enabled

### Browser Extension Test Enabled Disabled Disabled Enabled

### Security Review Enabled Enabled Enabled Enabled

### Sub-agent Matrix

### Sub-agent Claude Code Codex

### Code Reviewer Enabled Enabled

### Debugger Enabled Enabled

### Security Auditor Enabled Disabled

### Frontend Architect Enabled Disabled

### Pi Resources

### Pi Resources

### - Skills: 4

### - Prompt Templates: 2

### - Extensions: 1

### - Packages: 1

# 11.5 项目模块核心操作

1 添加项目。
2 删除项目。
3 重新扫描项目。
4 查看项目 Agent 配置。
5 给项目启用 MCP。
6 给项目启用 Skill。
7 给项目启用 Sub-agent。
8 查看 Pi Resources。
9 查看配置风险。
10 打开项目目录。

⠀12. MCP Servers 模块
12.1 页面目标
统一管理 Claude Code、Codex、opencode 的 MCP servers。
opencode 官方文档说明，MCP servers 可以定义在 OpenCode Config 的 mcp 字段下，每个 MCP server 有唯一名称，并支持 enabled 配置。

# 12.2 MCP 列表页

展示字段：
1 MCP 名称。
2 类型。
3 适用 Agent。
4 适用范围：全局 / 项目。
5 是否启用。
6 健康状态。
7 最近修改时间。
8 标签。

⠀12.3 MCP 详情页
展示：
1 MCP 名称。
2 描述。
3 连接方式。
4 所属分类。
5 绑定的 Agents。
6 绑定的项目。
7 是否启用。
8 配置风险。
9 最近修改记录。

⠀12.4 MCP 核心功能
1 添加 MCP。
2 编辑 MCP。
3 删除 MCP。
4 启用 MCP。
5 禁用 MCP。
6 复制到其他 Agent。
7 复制到项目。
8 从已有配置导入 MCP。
9 查看 MCP 使用位置。
10 检查 MCP 是否存在风险。

⠀12.5 MCP 模板
MVP 可以内置常见 MCP 模板：
1 GitHub
2 Filesystem
3 Playwright
4 Context7
5 Chrome DevTools
6 Figma
7 SQLite
8 Postgres
9 Browser Tools
10 Notion

⠀13. Skills 模块
13.1 页面目标
建立一个统一的 Skill Library。
用户不需要分别去 Claude、Codex、opencode、Pi 的目录里管理 Skills，而是在 AgentHub 中统一创建、维护、分类和启用。
Codex 官方文档说明，Skills 可以给 Codex 增加任务特定能力，skill 会包含 instructions、resources 和 optional scripts。
Pi 文档也说明其 skills 是 capability package，可用于扩展工作流。

# 13.2 Skill Library 页面

展示：
1 Skill 名称。
2 描述。
3 标签。
4 适用 Agent。
5 已同步目标数量。
6 状态：草稿 / 已启用 / 已归档。
7 最近修改时间。

⠀13.3 Skill 详情页
展示：
1 Skill 名称。
2 Skill 描述。
3 使用场景。
4 适用 Agent。
5 关联项目。
6 同步状态。
7 包含的参考资料。
8 包含的模板。
9 包含的辅助脚本。

⠀13.4 Skill 核心功能
1 创建 Skill。
2 编辑 Skill。
3 导入已有 Skill。
4 删除 Skill。
5 归档 Skill。
6 给 Skill 添加标签。
7 将 Skill 启用到指定 Agent。
8 将 Skill 启用到指定项目。
9 查看 Skill 被哪些项目使用。
10 检查 Skill 是否缺少必要说明。

⠀13.5 Skill 使用策略
AgentHub 中的 Skill Library 是主要管理入口。
对不同 Agent 的处理方式：
1 Claude Code：从 AgentHub 同步过去。
2 Codex：从 AgentHub 同步过去。
3 opencode：从 AgentHub 同步或适配其 skills 机制。
4 Pi：优先引用 AgentHub 的 Skill Library，而不是复制多份。

⠀Pi settings 文档说明，Pi 可以通过 settings 中的资源路径加载 skills、prompts、extensions 和 themes，并支持绝对路径与 ~。

# 14. Sub-agents 模块

# 14.1 页面目标

统一管理 Claude Code 和 Codex 的 Sub-agents / Custom Agents。
Sub-agent 用于让不同专家 agent 负责不同类型任务，而不是让一个主 agent 处理所有事情。

# 14.2 典型 Sub-agent 类型

MVP 内置一些模板：
1 Code Reviewer
2 Debugger
3 Security Auditor
4 Frontend Architect
5 Test Writer
6 Refactor Planner
7 Documentation Reviewer
8 Product Requirement Reviewer
9 Codebase Explorer
10 Performance Analyst

⠀14.3 Sub-agent Library 页面
展示：
1 Sub-agent 名称。
2 描述。
3 适用 Agent。
4 标签。
5 绑定的 MCP。
6 绑定的 Skills。
7 已启用目标。
8 最近修改时间。

⠀14.4 Sub-agent 详情页
展示：
1 Sub-agent 名称。
2 角色描述。
3 适合处理的任务。
4 不适合处理的任务。
5 关联 Skills。
6 关联 MCP。
7 适用 Agent。
8 启用范围：全局 / 项目。
9 当前启用项目。
10 风险提醒。

⠀14.5 创建 Sub-agent 流程

### Step 1：基础信息

用户填写：
1 名称。
2 描述。
3 标签。
4 适用 Agent。

⠀Step 2：角色定义
用户填写：
1 这个 Sub-agent 是什么角色。
2 应该处理什么任务。
3 不应该处理什么任务。
4 输出应该是什么形式。

⠀Step 3：能力绑定
用户选择：
1 可用 MCP。
2 可用 Skills。
3 适合的任务类型。
4 是否只读。
5 是否适合修改代码。

⠀Step 4：启用范围
用户选择：
1 全局启用。
2 只在某个项目启用。
3 同步到 Claude Code。
4 同步到 Codex。

⠀14.6 Sub-agent 核心功能
1 创建 Sub-agent。
2 编辑 Sub-agent。
3 删除 Sub-agent。
4 归档 Sub-agent。
5 启用到 Claude Code。
6 启用到 Codex。
7 启用到指定项目。
8 绑定 MCP。
9 绑定 Skills。
10 查看使用位置。
11 检测同名冲突。
12 从模板快速创建。

⠀14.7 Pi 与 Sub-agent 的关系
Pi 当前不作为 Sub-agent 管理对象。
Pi 官方介绍中也明确强调其核心保持轻量，通过 extensions、skills、prompt templates、themes 和 packages 扩展；Pi 官网还提到 Pi skips features like sub-agents and plan mode。
因此 MVP 中：
1 Pi 不进入 Sub-agent Manager。
2 Pi 单独进入 Pi Resources 模块。
3 如果未来 Pi 增加原生 Sub-agent，再补充支持。

⠀15. Pi Resources 模块
15.1 页面目标
针对 Pi coding agent 提供专门的资源管理页面。
Pi 不强行套入 MCP / Sub-agent 模型，而是按照 Pi 自身的扩展体系管理。

# 15.2 Pi Resources 包含内容

1 Settings
2 Skills
3 Prompt Templates
4 Extensions
5 Packages
6 Themes

⠀Pi packages 可用于打包 extensions、skills、prompt templates 和 themes，也支持 global / project scope 下启用或禁用资源。

# 15.3 Pi Overview 页面

展示：
1 Pi 是否安装。
2 Pi 版本。
3 全局配置是否存在。
4 当前项目配置是否存在。
5 Skills 数量。
6 Prompt Templates 数量。
7 Extensions 数量。
8 Packages 数量。
9 当前风险提醒。

⠀示例：

### Pi Coding Agent

### Status: Installed

### Global Resources:

### - Skills: 8

### - Prompt Templates: 4

### - Extensions: 2

### - Packages: 1

### Project Resources:

### - Skills: 4

### - Prompt Templates: 2

### - Extensions: 1

### - Packages: 0

### Warnings:

### - One extension may execute local commands

### - Project settings override default model

# 15.4 Pi Settings 页面

展示和管理：
1 默认模型。
2 Thinking level。
3 是否隐藏 thinking block。
4 Session 相关设置。
5 Compaction 设置。
6 Retry 设置。
7 Resource paths。
8 是否启用 Skill commands。
9 Terminal / Images 相关设置。

⠀15.5 Pi Skills 页面
功能：
1 查看 Pi 当前加载的 Skills。
2 查看 Skills 来源：全局 / 项目 / package / AgentHub。
3 将 AgentHub Skill Library 引用到 Pi。
4 禁用某个 Skill。
5 检查 Skill 路径是否有效。
6 查看某个 Skill 被哪些项目使用。

⠀15.6 Pi Prompt Templates 页面
Pi Prompt Templates 和通用 Prompt Library 分开。
通用 Prompt Library 只复制使用。
Pi Prompt Templates 用于 Pi slash command 场景。
功能：
1 查看 Pi prompt templates。
2 创建 Pi prompt template。
3 编辑 Pi prompt template。
4 启用到全局。
5 启用到项目。
6 从 Pi 导入已有 prompt template。
7 复制为普通 prompt。

⠀15.7 Pi Extensions 页面
Pi Extensions 是 Pi 的重要扩展能力。
功能：
1 查看全局 extensions。
2 查看项目 extensions。
3 启用 extension。
4 禁用 extension。
5 查看 extension 来源。
6 风险提醒。
7 标记不可信 extension。
8 打开 extension 文件位置。

⠀15.8 Pi Packages 页面
功能：
1 查看已安装 packages。
2 查看 package 来源。
3 查看 package 提供了哪些 resources。
4 启用 package 中的 skills。
5 启用 package 中的 prompts。
6 启用 package 中的 extensions。
7 禁用 package。
8 移除 package。
9 风险提醒。

⠀16. Prompts 模块
16.1 页面目标
提供一个轻量 Prompt 模板库，方便用户沉淀和复制常用 prompts。

# 16.2 产品边界

Prompt 模板只做：
1 创建。
2 编辑。
3 分类。
4 搜索。
5 收藏。
6 变量填写。
7 预览。
8 复制。

⠀不做：
1 不自动写入项目文件。
2 不自动修改 Claude / Codex / opencode / Pi 配置。
3 不和 Agent 的规则文件强绑定。

⠀16.3 Prompt 分类建议
1 Code Review
2 Bug Investigation
3 Refactor
4 Architecture Design
5 UI Design Agent
6 PRD to Tasks
7 Security Audit
8 Project Audit
9 Documentation
10 Testing
11 Wallet Development
12 Trading System

⠀16.4 Prompt 使用流程

### 选择 Prompt 模板

### ↓

### 填写变量

### ↓

### 预览最终 Prompt

### ↓

### 复制到剪贴板

### ↓

### 粘贴到任意 Coding Agent 中使用

# 17. Agents 模块

# 17.1 页面目标

展示 AgentHub 当前识别到的所有 Coding Agents。

# 17.2 Agent 列表

展示：
1 Agent 名称。
2 是否安装。
3 版本。
4 支持能力。
5 全局配置状态。
6 项目配置状态。
7 最近扫描时间。

⠀17.3 Agent 详情页
以 Claude Code 为例：

### Claude Code

### Status: Installed

### Capabilities:

### - MCP

### - Skills

### - Sub-agents

### - Project Config

### Global Resources:

### - MCP: 12

### - Skills: 8

### - Sub-agents: 5

### Actions:

### - Rescan

### - Open Config Location

### - View Related Projects

### - Run Doctor

# 18. Doctor 健康检查模块

# 18.1 页面目标

帮助用户发现配置问题和潜在风险。

# 18.2 检查类型

### Agent 检查

1 Agent 是否安装。
2 配置是否可读取。
3 配置是否可写入。
4 项目配置是否存在。
5 配置是否被多个 scope 覆盖。

⠀MCP 检查
1 MCP 是否重复。
2 MCP 是否缺少环境变量。
3 MCP 是否被多个 Agent 重复配置。
4 MCP 是否被禁用。
5 MCP 是否有潜在危险命令。
6 MCP 是否存在明文 secret。

⠀Skill 检查
1 Skill 是否缺少说明。
2 Skill 是否缺少入口文件。
3 Skill 是否已经过期。
4 Skill 是否被外部修改。
5 Skill 是否没有任何使用项目。

⠀Sub-agent 检查
1 Sub-agent 是否缺少描述。
2 Sub-agent 是否同名冲突。
3 Sub-agent 绑定的 MCP 是否不存在。
4 Sub-agent 绑定的 Skill 是否不存在。
5 Sub-agent 是否权限过大。

⠀Pi Resources 检查
1 Pi resource path 是否存在。
2 Pi package 是否重复。
3 Pi extension 是否可信。
4 Pi project settings 是否覆盖 global settings。
5 Pi skills / prompts 是否加载失败。

⠀18.3 风险级别

### Info

### Warning

### Critical

示例：

### Critical:

### - GitHub MCP contains raw token

### Warning:

### - Project Codex config overrides global MCP "filesystem"

### Info:

### - Skill "react-refactor" is not used by any project

# 19. Backups 模块

# 19.1 页面目标

让用户对配置修改有安全感。

# 19.2 功能

1 查看备份历史。
2 查看某次修改影响了哪些配置。
3 查看修改前后差异。
4 恢复单个配置。
5 恢复某个项目的配置。
6 恢复某个 Agent 的全局配置。
7 删除旧备份。

⠀20. Settings 模块
20.1 App Settings
管理：
1 AgentHub 数据目录。
2 默认备份策略。
3 是否开启配置风险提醒。
4 是否开启 secret 检查。
5 是否开启危险命令检查。
6 是否默认扫描项目配置。
7 主题。
8 是否显示高级功能。
9 是否允许实验性 agent adapter。

⠀20.2 Agent Settings
每个 agent 可以配置：
1 是否启用该 agent。
2 配置位置。
3 是否自动扫描。
4 是否允许写入配置。
5 默认作用域：全局 / 项目。
6 是否开启健康检查。

⠀21. MVP 功能范围
21.1 P0：必须实现

### 基础能力

1 macOS Desktop App。
2 本地数据管理。
3 添加项目。
4 扫描项目。
5 展示 Dashboard。

⠀Agent 支持
1 Claude Code。
2 Codex。
3 opencode。
4 Pi coding agent。

⠀MCP Manager
1 展示 MCP。
2 添加 MCP。
3 编辑 MCP。
4 删除 MCP。
5 启用 / 禁用 MCP。
6 应用到全局。
7 应用到项目。
8 查看 MCP Matrix。

⠀Sub-agent Manager
1 创建 Sub-agent。
2 管理 Claude Code Sub-agent。
3 管理 Codex Custom Agent。
4 启用到全局。
5 启用到项目。
6 查看 Sub-agent Matrix。

⠀Pi Resources
1 查看 Pi Settings。
2 查看 Pi Skills。
3 查看 Pi Prompt Templates。
4 查看 Pi Extensions。
5 查看 Pi Packages。
6 将 AgentHub Skill Library 引用给 Pi。

⠀Prompts
1 创建 Prompt。
2 编辑 Prompt。
3 分类。
4 搜索。
5 变量填写。
6 预览。
7 复制。

⠀Doctor
1 MCP 重复检查。
2 缺失 env 检查。
3 明文 secret 检查。
4 Skill 缺失检查。
5 Sub-agent 冲突检查。
6 Pi resource path 检查。

⠀Backups
1 修改前备份。
2 查看备份。
3 恢复备份。

⠀21.2 P1：第二阶段
1 Skill 编辑器。
2 Skill 导入。
3 Skill 使用统计。
4 Skill 同步状态。
5 Pi Prompt Templates 编辑。
6 Pi Extensions 管理。
7 Pi Packages 启用 / 禁用。
8 Sub-agent 模板库。
9 MCP 模板库增强。
10 配置变更历史。

⠀21.3 P2：后续增强
1 Profiles 一键应用。
2 多项目批量操作。
3 Skill Gallery。
4 Sub-agent Gallery。
5 MCP Template Marketplace。
6 Pi Package 安装辅助。
7 Open-source plugin adapter。
8 更多 agent 支持。
9 多设备同步。
10 团队共享。

⠀22. 推荐 MVP 首页功能优先级
第一版不要做太大，建议聚焦：

### 1. Projects

### 2. MCP Servers

### 3. Sub-agents

### 4. Pi Resources

### 5. Prompts

### 6. Doctor

其中最重要的是：

### Project Matrix + MCP Manager + Sub-agent Manager + Pi Resource Viewer

这几个功能能最快解决你的实际痛点。

# 23. 核心用户流程

# 23.1 首次启动

### 打开 AgentHub Local

### ↓

### 检测本机已安装的 Coding Agents

### ↓

### 导入已有全局配置

### ↓

### 添加项目目录

### ↓

### 扫描项目中的 Agent 配置

### ↓

### 展示 Dashboard 和 Project Matrix

# 23.2 给项目添加 MCP

### 进入 MCP Servers

### ↓

### 选择或新建 MCP

### ↓

### 选择目标 Agent

### ↓

### 选择目标项目

### ↓

### 预览影响范围

### ↓

### 确认应用

### ↓

### 完成

# 23.3 创建 Sub-agent

### 进入 Sub-agents

### ↓

### 选择模板或新建

### ↓

### 填写角色说明

### ↓

### 绑定 MCP / Skills

### ↓

### 选择 Claude Code / Codex

### ↓

### 选择全局或项目

### ↓

### 确认启用

# 23.4 给 Pi 启用 Skill

### 进入 Pi Resources

### ↓

### 选择 Skills

### ↓

### 选择 AgentHub Skill Library 中的 Skill

### ↓

### 选择全局或项目

### ↓

### 确认启用

# 23.5 使用 Prompt 模板

### 进入 Prompts

### ↓

### 选择模板

### ↓

### 填写变量

### ↓

### 预览

### ↓

### 复制

### ↓

### 粘贴到任意 Coding Agent

# 24. 产品边界总结

AgentHub Local 要做的是：
1 统一管理 Agent 配置。
2 统一管理 MCP。
3 统一管理 Skills。
4 统一管理 Sub-agents。
5 统一管理 Pi Resources。
6 统一管理 Prompt 模板。
7 提供项目级可视化。
8 提供风险检查。
9 提供备份和回滚。

⠀AgentHub Local 不做的是：
1 不替代 Coding Agent。
2 不做 Chat。
3 不执行代码任务。
4 不托管 MCP。
5 不做云端协作。
6 不做复杂 workflow 编排。
7 不自动写入通用 Prompt 到项目规则文件。

## 25. 最终产品总结

AgentHub Local 的核心价值是：
让程序员能够在一个本地桌面应用中，看清楚、管得住、复用好自己所有 Coding Agent 的 MCP、Skills、Sub-agents、Pi Resources 和 Prompt 模板。
它不是一个通用低代码平台，也不是 Agent Chat 工具，而是一个更偏开发者基础设施的本地管理工具。
最适合的 MVP 方向是：

### macOS Desktop App

### + Project Matrix

### + MCP Manager

### + Sub-agent Manager

### + Unified Skill Library

### + Pi Resource Viewer

### + Prompt Copy Library

### + Doctor

### + Backup
