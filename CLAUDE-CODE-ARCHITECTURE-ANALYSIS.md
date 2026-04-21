# Claude Code 源代码架构详细分析报告

> 分析对象：`claude-code源代码/` 目录（TypeScript 版本）
>
> 分析日期：2026-04-21
>
> 分析方式：子代理并行分析 + 综合汇总

---

## 目录

1. [项目概述](#1-项目概述)
2. [入口点和启动流程](#2-入口点和启动流程)
3. [QueryEngine 核心引擎](#3-queryengine-核心引擎)
4. [工具系统](#4-工具系统)
5. [命令系统](#5-命令系统)
6. [Bridge 系统](#6-bridge-系统)
7. [UI 组件和状态管理](#7-ui-组件和状态管理)
8. [服务层和插件系统](#8-服务层和插件系统)
9. [整体架构总结](#9-整体架构总结)

---

## 1. 项目概述

### 1.1 项目基本信息

| 属性 | 值 |
|------|-----|
| **语言** | TypeScript (严格模式) |
| **运行时** | Bun |
| **终端 UI** | React + Ink |
| **规模** | ~1,900 个文件，512,000+ 行代码 |
| **CLI 解析** | Commander.js |
| **Schema 验证** | Zod v4 |
| **代码搜索** | ripgrep |
| **协议** | MCP SDK, LSP |

### 1.2 目录结构

```
src/
├── entrypoints/
│   ├── cli.tsx           # 启动入口（快速路径）
│   └── init.ts           # 初始化逻辑
├── main.tsx              # 主应用入口
├── commands.ts           # 命令注册中心
├── tools.ts              # 工具注册中心
├── Tool.ts               # 工具类型定义
├── QueryEngine.ts        # LLM 查询引擎（~46K 行）
├── context.ts            # 上下文收集
├── cost-tracker.ts       # 成本追踪
│
├── commands/             # Slash 命令实现 (~50 个)
├── tools/                # 工具实现 (~40 个)
├── components/           # Ink UI 组件 (~140 个)
├── hooks/                # React hooks (~150 个)
├── services/             # 外部服务集成
├── screens/              # 全屏 UI (REPL, Doctor, Resume)
├── types/                # TypeScript 类型定义
├── utils/                # 实用函数
│
├── bridge/               # IDE 和远程控制桥接
├── coordinator/          # 多代理协调器
├── plugins/              # 插件系统
├── skills/               # 技能系统
├── keybindings/          # 键绑定配置
├── vim/                  # Vim 模式
├── voice/                # 语音输入
├── remote/               # 远程会话
├── server/               # 服务器模式
├── memdir/               # 持久化内存目录
├── tasks/                # 任务管理
├── state/                # 状态管理
├── migrations/           # 配置迁移
├── schemas/              # 配置 Schema (Zod)
└── ...
```

---

## 2. 入口点和启动流程

### 2.1 启动入口：`entrypoints/cli.tsx`

**设计理念：快速路径（Fast Path）**

```typescript
// 多个快速路径分支，避免加载完整模块
if (process.argv.includes('--version')) {
  console.log(version)
  process.exit(0)
}

if (process.argv.includes('--dump-system-prompt')) {
  // 直接输出，不加载 main.tsx
  console.log(await buildSystemPrompt(...))
  process.exit(0)
}

// bridge 模式、daemon 模式等都有独立快速路径

// 最后才加载完整 CLI
await import('../main.js')
```

### 2.2 主初始化流程

```
entrypoints/cli.tsx
    ↓
（快速路径检查）
    ↓
main.tsx
    ↓
entrypoints/init.ts (init() 函数)
    ├─ enableConfigs() - 启用配置系统
    ├─ applySecurityEnvironmentVariables() - 安全环境变量
    ├─ setupGracefulShutdown() - 优雅关闭
    ├─ initializeFirstPartyEventLogging() - 事件日志
    ├─ configureNetwork() - 网络配置（mTLS、代理）
    ├─ preconnectToAnthropicAPI() - 预连接 API
    └─ initializeTempDir() - 临时目录
    ↓
启动性能分析 (profileCheckpoint)
    ↓
运行数据迁移 (runMigrations)
    ↓
初始化应用状态 (AppState)
    ↓
加载命令和工具
    ↓
启动 REPL 界面 (launchRepl)
```

### 2.3 命令注册系统：`commands.ts`

**核心函数：**

| 函数 | 职责 |
|------|------|
| `COMMANDS` (memoized) | 所有内置命令数组 |
| `getCommands(cwd)` | 获取当前用户可用的所有命令 |
| `findCommand(name, commands)` | 按名称查找命令 |
| `getSkillToolCommands(cwd)` | 获取可被模型调用的技能命令 |
| `meetsAvailabilityRequirement(cmd)` | 检查命令是否满足认证/提供商要求 |

**命令加载层次：**

```
1. Bundled Skills（打包技能）
   ↓
2. Builtin Plugin Skills（内置插件技能）
   ↓
3. Skill Dir Commands（技能目录命令）
   ↓
4. Workflow Commands（工作流命令）
   ↓
5. Plugin Commands（插件命令）
   ↓
6. Plugin Skills（插件技能）
   ↓
7. Builtin Commands（内置命令）- 优先级最低
```

### 2.4 工具注册系统：`tools.ts`

**核心函数：**

| 函数 | 职责 |
|------|------|
| `getAllBaseTools()` | 获取所有可能可用的基础工具 |
| `getTools(permissionContext)` | 获取当前权限上下文可用的工具 |
| `assembleToolPool(permissionContext, mcpTools)` | 组装完整工具池（内置 + MCP） |
| `filterToolsByDenyRules(tools, permissionContext)` | 根据拒绝规则过滤工具 |

**工具加载策略：**
1. 特性标志（Feature Flags）- 编译时死代码消除
2. 环境条件 - `process.env.USER_TYPE === 'ant'` 控制内部工具
3. 启用检查 - 每个工具都有 `isEnabled()` 方法
4. 权限过滤 - 通过 `filterToolsByDenyRules()` 过滤

---

## 3. QueryEngine 核心引擎

### 3.1 QueryEngine 类结构

```typescript
export class QueryEngine {
  private config: QueryEngineConfig
  private mutableMessages: Message[]
  private abortController: AbortController
  private permissionDenials: SDKPermissionDenial[]
  private totalUsage: NonNullableUsage
  private hasHandledOrphanedPermission = false
  private readFileState: FileStateCache
  private discoveredSkillNames = new Set<string>()
  private loadedNestedMemoryPaths = new Set<string>()
}
```

### 3.2 LLM API 调用机制

**调用流程：**

```
QueryEngine.submitMessage()
    ↓
query() (query.ts)
    ↓
queryModel() (claude.ts)
    ↓
withRetry() (withRetry.ts)
```

**关键模块位置：**
- **API 客户端**: `src/services/api/claude.ts`
- **重试机制**: `src/services/api/withRetry.ts`
- **查询主循环**: `src/query.ts`

**API 调用特点：**
- 支持流式和非流式两种模式
- 使用 `@anthropic-ai/sdk` 的 Beta API
- 支持 prompt caching（提示缓存）
- 支持多种模型提供商（1P、Bedrock、Vertex）

### 3.3 流式响应处理

**核心处理函数：`queryModel()` (claude.ts)**

**流式事件类型：**
- `message_start` - 消息开始
- `content_block_start` - 内容块开始
- `content_block_delta` - 内容块增量
- `content_block_stop` - 内容块停止
- `message_delta` - 消息增量
- `message_stop` - 消息停止

**流式看门狗：**
```
streamWatchdog
├─ idle 警告定时器
├─ idle 中止定时器
└─ 实时重置机制（收到任何 chunk 时）
```

### 3.4 工具调用循环 (Tool-use Loop)

**位置：**
- **入口**: `src/query.ts` - `queryLoop()`
- **编排**: `src/services/tools/toolOrchestration.ts` - `runTools()`
- **执行**: `src/services/tools/toolExecution.ts` - `runToolUse()`

**工具调用流程：**

```
queryLoop (无限循环)
├─ 调用 queryModel() 获取 LLM 响应
├─ 检测 tool_use 内容块
├─ 调用 runTools() 执行工具
│  ├─ partitionToolCalls() - 分批
│  │  ├─ 并发安全工具（只读）→ 并发执行
│  │  └─ 非并发安全工具 → 串行执行
│  ├─ runToolsConcurrently() / runToolsSerially()
│  └─ yield 工具结果
├─ 将工具结果作为 user message 加入
└─ 继续循环 (直到 stop_reason: end_turn)
```

**并发控制：**
- **最大并发数**: 由 `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY` 环境变量控制（默认 10）
- **并发安全判断**: `tool.isConcurrencySafe(input)`

### 3.5 思考模式 (Thinking Mode)

**配置类型 (src/utils/thinking.ts)：**
```typescript
export type ThinkingConfig =
  | { type: 'adaptive' }           // 自适应思考
  | { type: 'enabled'; budgetTokens: number }  // 固定预算
  | { type: 'disabled' }           // 禁用
```

**思考规则：**
1. 包含 thinking/redacted_thinking 块的消息必须有 `max_thinking_length > 0`
2. Thinking 块不能是消息中的最后一个块
3. Thinking 块在整个助手轨迹期间必须保留

### 3.6 重试逻辑

**位置**: `src/services/api/withRetry.ts`

**重试配置：**
- **默认最大重试次数**: 10 次
- **基准延迟**: 500ms
- **529 错误最大重试**: 3 次

**可重试错误判断 (`shouldRetry()`)：**
- 408 (请求超时)
- 409 (锁超时)
- 429 (速率限制，非订阅用户)
- 5xx (服务器错误)
- `x-should-retry: true` 头
- 连接错误

**退避策略：**
```
延迟 = min(BASE_DELAY_MS * 2^(attempt-1), maxDelayMs) + 随机抖动
```

**特殊重试模式：**

1. **持久重试模式** (`CLAUDE_CODE_UNATTENDED_RETRY`):
   - 无限重试 429/529
   - 最大退避: 5 分钟
   - 心跳间隔: 30 秒

2. **Fast Mode 降级**:
   - 429/529 时可切换到标准速度

3. **529 错误模型降级**:
   - 连续 3 次 529 → 切换到 fallback model

4. **Context Overflow 调整**:
   - 检测 400 错误中的 context overflow
   - 自动调整 `max_tokens` 重试

### 3.7 Token 计数和成本追踪

**Token 计数 (src/utils/tokens.ts)：**
```typescript
// 从 API 响应获取
getTokenUsage(message)
getTokenCountFromUsage(usage)  // input + cache + output

// 从消息历史获取
tokenCountFromLastAPIResponse(messages)
finalContextTokensFromLastResponse(messages)

// 估算
roughTokenCountEstimationForMessages()
```

**成本追踪 (src/cost-tracker.ts)：**

**追踪数据：**
- `totalCostUSD` - 总成本
- `totalInputTokens` - 输入 tokens
- `totalOutputTokens` - 输出 tokens
- `totalCacheReadInputTokens` - 缓存读取 tokens
- `totalCacheCreationInputTokens` - 缓存创建 tokens
- `totalAPIDuration` - API 总时长
- `modelUsage` - 按模型分类的使用情况

### 3.8 消息流转过程

```
用户输入
    │
    ▼
QueryEngine.submitMessage()
├─ 构建 system prompt
├─ 处理用户输入 processUserInput()
└─ 持久化用户消息
    │
    ▼
query() 函数 (query.ts)
queryLoop() 无限循环:
│
├─ 消息预处理
│  ├─ Snip compaction
│  ├─ Microcompact
│  ├─ Auto-compact
│  └─ Context collapse
│      │
│      ▼
│  queryModel() 调用 LLM
│  ├─ 流式响应处理
│  └─ withRetry() 重试
│      │
│      ▼
│  检查 tool_use
│  有? ──是──► runTools()
│  │        执行工具并 yield
│  │        结果加入消息
│  └──否───────┐
└──────────────┼───────────────┘
               ▼
   stop_reason == end_turn?
   是 → 退出循环
   否 → 继续下一轮
    │
    ▼
返回最终结果
- 总成本
- Token 使用量
- 模型使用情况
- 权限拒绝
```

---

## 4. 工具系统

### 4.1 工具基类和接口定义

**核心 Tool 接口：**

```typescript
type Tool<Input extends AnyObject = AnyObject, Output = unknown> = {
  readonly name: string
  aliases?: string[]
  searchHint?: string
  readonly inputSchema: Input
  outputSchema?: z.ZodType<unknown>
  readonly maxResultSizeChars: number
  readonly shouldDefer?: boolean
  readonly alwaysLoad?: boolean
  readonly strict?: boolean
  isMcp?: boolean
  isLsp?: boolean

  // 核心执行方法
  call(args, context, canUseTool, parentMessage, onProgress?): Promise<ToolResult<Output>>

  // 权限和验证
  checkPermissions(input, context): Promise<PermissionResult>
  validateInput?(input, context): Promise<ValidationResult>

  // 特性标识
  isEnabled(): boolean
  isConcurrencySafe(input): boolean
  isReadOnly(input): boolean
  isDestructive?(input): boolean

  // UI 渲染方法
  prompt(options): Promise<string>
  description(input, options): Promise<string>
  userFacingName(input): string
  renderToolUseMessage(input, options): React.ReactNode
  renderToolResultMessage(content, progress, options): React.ReactNode
}
```

**默认值（TOOL_DEFAULTS）：**
- `isEnabled()` → `true`
- `isConcurrencySafe()` → `false`（默认非线程安全）
- `isReadOnly()` → `false`（默认有写入操作）
- `isDestructive()` → `false`（默认非破坏性）
- `checkPermissions()` → `{ behavior: 'allow', updatedInput }`（默认允许）

### 4.2 核心工具清单

| 工具名称 | 功能描述 | 只读 | 并发安全 |
|---------|---------|------|---------|
| **AgentTool** | 运行子代理、fork 代理、管理代理会话 | 否 | 否 |
| **TaskOutputTool** | 输出任务结果（内部工具） | 否 | 否 |
| **BashTool** | 执行 Bash 命令 | 视命令而定 | 否 |
| **GlobTool** | 文件模式匹配搜索 | 是 | 是 |
| **GrepTool** | 文件内容搜索 | 是 | 是 |
| **FileReadTool** | 读取文件、图像、PDF、Notebook | 是 | 是 |
| **FileEditTool** | 编辑文件内容 | 否 | 否 |
| **FileWriteTool** | 写入新文件 | 否 | 否 |
| **NotebookEditTool** | 编辑 Jupyter Notebook | 否 | 否 |
| **WebFetchTool** | 获取网页内容 | 是 | 是 |
| **TodoWriteTool** | 编写待办事项 | 否 | 否 |
| **WebSearchTool** | 网络搜索 | 是 | 是 |
| **TaskCreateTool** | 创建任务（TodoV2） | 否 | 否 |
| **TaskGetTool** | 获取任务详情 | 是 | 是 |
| **TaskUpdateTool** | 更新任务 | 否 | 否 |
| **TaskListTool** | 列出任务 | 是 | 是 |
| **EnterWorktreeTool** | 进入 Git worktree | 否 | 否 |
| **ExitWorktreeTool** | 退出 Git worktree | 否 | 否 |
| **SendMessageTool** | 发送消息（团队协作） | 否 | 否 |
| **TeamCreateTool** | 创建团队 | 否 | 否 |
| **TeamDeleteTool** | 删除团队 | 否 | 否 |
| **ListMcpResourcesTool** | 列出 MCP 资源 | 是 | 是 |
| **ReadMcpResourceTool** | 读取 MCP 资源 | 是 | 是 |
| **MCPTool** | 调用 MCP 工具 | 视工具而定 | 否 |
| **ToolSearchTool** | 搜索可用工具 | 是 | 是 |
| **PowerShellTool** | 执行 PowerShell 命令 | 视命令而定 | 否 |
| **ScheduleCronTool** | 定时任务管理 | 否 | 否 |
| **RemoteTriggerTool** | 远程触发（Agent 触发器） | 否 | 否 |
| **WorkflowTool** | 工作流脚本 | 否 | 否 |
| **SleepTool** | 休眠/延迟 | 是 | 是 |
| **SyntheticOutputTool** | 合成输出 | 否 | 否 |
| **McpAuthTool** | MCP 认证 | 否 | 否 |

### 4.3 权限模型

**权限模式类型：**
```typescript
type PermissionMode =
  | 'default'           // 默认模式：询问用户
  | 'acceptEdits'       // 自动接受编辑操作
  | 'bypassPermissions' // 绕过所有权限检查
  | 'dontAsk'           // 不询问，自动拒绝
  | 'plan'              // 计划模式
  | 'auto'              // 自动模式（使用分类器）
  | 'bubble'            // 冒泡模式
```

**权限规则：**
```typescript
type PermissionRule = {
  source: PermissionRuleSource  // userSettings | projectSettings | ...
  ruleBehavior: PermissionBehavior  // 'allow' | 'deny' | 'ask'
  ruleValue: {
    toolName: string
    ruleContent?: string  // 可选的模式内容（如 "git *"）
  }
}
```

**权限决策结果：**
```typescript
type PermissionResult =
  | { behavior: 'allow'; updatedInput?: Input; ... }
  | { behavior: 'deny'; message: string; ... }
  | { behavior: 'ask'; message: string; ... }
  | { behavior: 'passthrough'; message: string; ... }
```

**决策原因：**
- `rule` - 匹配到权限规则
- `mode` - 权限模式决定
- `hook` - 钩子决定
- `classifier` - 自动分类器决定
- `asyncAgent` - 异步代理决定
- `sandboxOverride` - 沙箱覆盖
- `safetyCheck` - 安全检查
- `workingDir` - 工作目录限制

### 4.4 工具执行流程

```
runToolUse()
  ↓
streamedCheckPermissionsAndCallTool()
  ↓
checkPermissionsAndCallTool()
  ├─ 1. Zod 输入验证
  ├─ 2. 工具 validateInput() 验证
  ├─ 3. 推测性启动分类器检查（BashTool）
  ├─ 4. 运行 PreToolUse 钩子
  ├─ 5. 权限检查（resolveHookPermissionDecision）
  │   ├─ 检查钩子权限结果
  │   ├─ 调用 canUseTool()
  │   └─ 返回 allow/deny/ask 决策
  ├─ 6. 如果允许 → 执行 tool.call()
  │   ├─ 记录开始时间和 telemetry
  │   ├─ 执行工具逻辑
  │   ├─ 处理进度回调 (onProgress)
  │   └─ 返回 ToolResult<Output>
  ├─ 7. 运行 PostToolUse 钩子
  ├─ 8. 映射结果到 API 格式
  └─ 9. 返回消息更新
```

---

## 5. 命令系统

### 5.1 命令的基类和接口定义

**核心类型定义：**

```typescript
// 命令基类定义
type CommandBase = {
  availability?: CommandAvailability[]  // 可用环境限制
  description: string                    // 命令描述
  name: string                           // 命令名称
  aliases?: string[]                     // 别名
  isEnabled?: () => boolean              // 启用检查
  isHidden?: boolean                     // 是否隐藏
  argumentHint?: string                  // 参数提示
  whenToUse?: string                     // 使用场景说明
  immediate?: boolean                    // 立即执行（绕过队列）
  isSensitive?: boolean                  // 参数是否敏感
}

// 命令的三种类型
type Command = CommandBase & (
  PromptCommand |        // 提示型命令（发送给模型）
  LocalCommand |         // 本地命令（返回文本）
  LocalJSXCommand        // 本地JSX命令（渲染UI）
)
```

### 5.2 三种命令类型详解

#### **PromptCommand（提示型命令）**
```typescript
type PromptCommand = {
  type: 'prompt'
  progressMessage: string
  contentLength: number
  source: SettingSource | 'builtin' | 'mcp' | 'plugin' | 'bundled'
  getPromptForCommand(args: string, context: ToolUseContext): Promise<ContentBlockParam[]>
  allowedTools?: string[]
  context?: 'inline' | 'fork'  // 执行方式：内联或子代理
  agent?: string                 // 子代理类型
  effort?: EffortValue
}
```

**特点**：不直接执行代码，而是生成提示文本发送给 Claude 模型处理。

#### **LocalCommand（本地命令）**
```typescript
type LocalCommand = {
  type: 'local'
  supportsNonInteractive: boolean
  load: () => Promise<LocalCommandModule>  // 懒加载
}

type LocalCommandCall = (
  args: string,
  context: LocalJSXCommandContext
) => Promise<LocalCommandResult>

type LocalCommandResult =
  | { type: 'text'; value: string }
  | { type: 'compact'; compactionResult: CompactionResult }
  | { type: 'skip' }
```

**特点**：直接在本地执行 TypeScript 代码，返回文本结果。

#### **LocalJSXCommand（本地JSX命令）**
```typescript
type LocalJSXCommand = {
  type: 'local-jsx'
  load: () => Promise<LocalJSXCommandModule>
}

type LocalJSXCommandCall = (
  onDone: LocalJSXCommandOnDone,
  context: ToolUseContext & LocalJSXCommandContext,
  args: string
) => Promise<React.ReactNode>
```

**特点**：渲染 React/Ink UI 组件，提供交互式界面。

### 5.3 关键命令实现

#### `/doctor` 命令（诊断工具）
- **类型**: `local-jsx`（渲染 UI）
- **功能**: 诊断和验证 Claude Code 安装和设置
- **组件**: `<Doctor />` 组件在 `src/screens/Doctor.tsx`

#### `/commit` 命令（Git 提交）
- **类型**: `prompt`（发送给模型）
- **功能**: 创建 git 提交
- **特点**: 包含 Git 安全协议（防止修改配置、跳过钩子等）

#### `/review` 和 `/ultrareview` 命令
- `/review` - 本地 PR 审查（prompt 类型）
- `/ultrareview` - 高级审查（local-jsx 类型，使用远程服务）

#### `/compact` 命令（上下文压缩）
- **类型**: `local`（返回特殊的 `compact` 结果类型）
- **功能**: 清除对话历史但保留摘要
- **支持多种压缩策略**: 会话内存压缩、反应式压缩、传统压缩

### 5.4 命令执行流程

```
用户输入 "/command args"
    ↓
parseSlashCommand() 解析
    ↓
hasCommand() 查找命令
    ↓
getMessagesForSlashCommand() 根据类型分发
    ↓
┌─────────────────┬──────────────────┬─────────────────────┐
│  PromptCommand  │  LocalCommand    │  LocalJSXCommand    │
│  发送给模型     │  执行本地代码    │  渲染 React 组件    │
└─────────────────┴──────────────────┴─────────────────────┘
    ↓
返回 messages 和 shouldQuery
```

---

## 6. Bridge 系统

### 6.1 Bridge 系统整体架构

Bridge 系统实现了 **Claude Code CLI 与 claude.ai Web IDE 之间的双向通信桥接**，支持远程控制、会话管理和工具执行。

**核心架构层次：**

```
┌─────────────────────────────────────────────────────────┐
│                    claude.ai Web IDE                     │
└──────────────────┬──────────────────────────────────────┘
                   │ WebSocket / SSE + REST API
┌──────────────────▼──────────────────────────────────────┐
│           Bridge Core (bridgeMain.ts)                    │
│  • Environment Registration                               │
│  • Work Polling Loop                                      │
│  • Session Lifecycle Management                           │
└──────────────────┬──────────────────────────────────────┘
                   │
    ┌──────────────┼──────────────┐
    │              │              │
┌───▼───┐    ┌───▼───┐    ┌────▼──────┐
│  REPL │    │Session│    │ Messaging │
│Bridge │    │Runner │    │ Protocol  │
└───┬───┘    └───┬───┘    └────┬──────┘
    │              │              │
┌───▼──────────────▼──────────────▼───────┐
│     Child Claude Code Process (Per-Session) │
└─────────────────────────────────────────────┘
```

### 6.2 关键文件职责

| 文件 | 职责 |
|------|------|
| **bridgeMain.ts** | 桥接主循环：环境注册、工作轮询、会话管理 |
| **bridgeMessaging.ts** | 消息协议：类型守卫、入站路由、服务器控制请求处理 |
| **bridgePermissionCallbacks.ts** | 权限回调：工具使用权限请求/响应处理 |
| **replBridge.ts** | REPL 会话桥接：与本地 REPL 的双向桥接 |
| **jwtUtils.ts** | JWT 认证：令牌解码、过期检查、自动刷新调度器 |
| **sessionRunner.ts** | 会话执行：子进程生成、活动监控、生命周期管理 |
| **types.ts** | 类型定义：配置、API、会话句柄等完整类型系统 |

### 6.3 通信协议和消息格式

**SDKMessage 类型** 作为基础消息格式：
- 类型守卫：`isSDKMessage()` 验证 type 字段
- 支持：`user`、`assistant`、`system`、`result` 等类型

**服务器发起的控制请求 (SDKControlRequest)：**
```typescript
{
  type: 'control_request',
  request_id: string,
  request: {
    subtype: 'initialize' | 'set_model' | 'interrupt' |
             'set_permission_mode' | 'set_max_thinking_tokens'
  }
}
```

**权限请求 (can_use_tool)：**
```typescript
// 来自子进程的权限请求
{
  type: 'control_request',
  request_id: string,
  request: {
    subtype: 'can_use_tool',
    tool_name: string,
    input: Record<string, unknown>,
    tool_use_id: string
  }
}
```

### 6.4 认证和安全机制

**JWT 令牌管理：**
- `decodeJwtPayload()` - JWT 解码（不验证签名）
- `decodeJwtExpiry()` - 提取过期时间
- `createTokenRefreshScheduler()` - 自动刷新调度器

**刷新策略：**
- **提前缓冲**: 在过期前 5 分钟触发刷新
- **回退间隔**: 30 分钟（当无法解码 JWT 过期时间时）
- **失败重试**: 最多 3 次，每次间隔 60 秒

**Work Secret 机制：**
```typescript
type WorkSecret = {
  version: 1,
  session_ingress_token: string,  // JWT for session ingress
  api_base_url: string,
  sources: Array<{type, git_info?}>,
  auth: Array<{type, token}>,
  claude_code_args?: Record<string, string>,
  mcp_config?: unknown,
  environment_variables?: Record<string, string>,
  use_code_sessions?: boolean  // CCR v2 标志
}
```

### 6.5 会话状态管理

**会话生成模式 (SpawnMode)：**
- `'single-session'` - 单一目录，会话结束后桥接退出
- `'worktree'` - 每个会话一个独立 git worktree
- `'same-dir'` - 所有会话共享同一目录

**会话句柄 (SessionHandle)：**
```typescript
type SessionHandle = {
  sessionId: string
  done: Promise<SessionDoneStatus>

  // 生命周期控制
  kill(): void           // SIGTERM
  forceKill(): void      // SIGKILL

  // 活动追踪
  activities: SessionActivity[]  // 环形缓冲区（最近 10 个）
  currentActivity: SessionActivity | null

  // 通信
  accessToken: string
  lastStderr: string[]
  writeStdin(data: string): void

  // 令牌更新
  updateAccessToken(token: string): void
}
```

---

## 7. UI 组件和状态管理

### 7.1 Ink UI 组件架构

**核心技术栈：**

- **Ink** - 用于构建命令行用户界面的 React 渲染器
- **核心组件**: `Box`、`Text`、`Button`、`Link`、`Newline`、`Spacer`
- **Hooks**: `useInput`、`useStdin`、`useTerminalFocus`、`useTabStatus`、`useAnimationFrame`

**自定义 Ink 组件系统：**

```
src/components/design-system/
├── ThemeProvider.tsx       // 主题上下文提供者
├── ThemedBox.tsx           // 带主题的布局容器
├── ThemedText.tsx          // 带主题的文本组件
└── color.ts                // 颜色定义
```

### 7.2 核心组件目录结构

```
src/components/
├── App.tsx                          # 应用根组件
├── PromptInput/                     # 提示输入模块（18个文件）
│   ├── PromptInput.tsx              # 主输入组件
│   ├── PromptInputFooter.tsx        # 底部栏
│   ├── PromptInputHelpMenu.tsx      # 帮助菜单
│   ├── HistorySearchInput.tsx       # 历史搜索
│   ├── Notifications.tsx            # 通知显示
│   ├── VoiceIndicator.tsx           # 语音模式指示器
│   └── usePromptInputPlaceholder.ts # 占位符钩子
├── LogoV2/                          # Logo 动画组件
├── FeedbackSurvey/                  # 反馈调查
├── CustomSelect/                    # 自定义选择器
├── permissions/                     # 权限相关组件
├── mcp/                             # MCP 协议组件
└── ... (~140个组件)
```

### 7.3 状态管理（AppState）

**核心文件：**
- `src/state/store.ts` - 简单的发布-订阅存储实现
- `src/state/AppStateStore.ts` - AppState 类型定义和默认值
- `src/state/AppState.tsx` - React 上下文和 hooks
- `src/state/selectors.ts` - 状态选择器（计算派生数据）
- `src/state/onChangeAppState.ts` - 状态变更副作用处理

**Store 实现：**
```typescript
type Store<T> = {
  getState: () => T
  setState: (updater: (prev: T) => T) => void
  subscribe: (listener: Listener) => () => void
}

export function createStore<T>(initialState: T, onChange?: OnChange<T>): Store<T>
```

**AppState 包含 60+ 个顶级字段，主要分为：**
- 核心设置（settings、verbose、mainLoopModel）
- UI 状态（expandedView、viewSelectionMode、footerSelection）
- Bridge 连接状态（replBridgeEnabled、replBridgeConnected 等）
- 任务和代理（tasks、agentNameRegistry、foregroundedTaskId）
- MCP 和插件（mcp.clients、mcp.tools、plugins.enabled 等）
- 权限系统（toolPermissionContext、denialTracking）
- 远程会话（remoteSessionUrl、remoteConnectionStatus 等）
- 推测执行（speculation、speculationSessionTimeSavedMs）

### 7.4 React Hooks 设计

**约 150+ 个 hooks，分类如下：**

| 类别 | 示例 |
|------|------|
| **状态访问类** | `useSettings()`、`useAppState(selector)`、`useSetAppState()` |
| **输入处理类** | `useTextInput()`、`useVimInput()`、`useArrowKeyHistory()`、`useHistorySearch()` |
| **Bridge 相关** | `useReplBridge()`、`useRemoteSession()`、`useDirectConnect()` |
| **通知系统** | `useStartupNotification()`、`useFastModeNotification()`、`usePluginInstallationStatus()` |
| **权限系统** | `useCanUseTool()`、`useSwarmPermissionPoller()` |
| **IDE 集成** | `useIDEIntegration()`、`useIdeConnectionStatus()`、`useIdeSelection()` |
| **工具类** | `useAfterFirstRender()`、`useTimeout()`、`useBlink()`、`useTerminalSize()` |

### 7.5 屏幕组件

| 组件 | 文件 | 功能 |
|------|------|------|
| **REPL** | `src/screens/REPL.tsx` | 主交互界面，处理对话流 |
| **Doctor** | `src/screens/Doctor.tsx` | 诊断和问题排查界面 |
| **ResumeConversation** | `src/screens/ResumeConversation.tsx` | 恢复会话界面 |

### 7.6 组件间数据流

```
main.tsx (入口)
  ├─ CLI 解析
  ├─ 初始化 Bootstrap 状态
  └─ launchRepl()
      └─ Ink render()
          └─ <App>
              ├─ <FpsMetricsProvider>
              ├─ <StatsProvider>
              └─ <AppStateProvider>
                  ├─ <MailboxProvider>
                  ├─ <VoiceProvider>
                  └─ <REPL> 或 <Doctor> 或 <ResumeConversation>
```

**状态更新流程：**
```
用户操作 (键盘/输入)
    ↓
事件处理函数 (useInput / useKeybindings)
    ↓
useSetAppState(updater)
    ↓
store.setState()
    ├─→ 对比新旧状态 (Object.is)
    ├─→ 调用 onChangeAppState() [副作用]
    └─→ 通知所有 listeners
        ↓
    useSyncExternalStore 触发
        ↓
    useAppState(selector) 重新计算
        ↓
    组件重新渲染 (仅依赖该切片的组件)
```

---

## 8. 服务层和插件系统

### 8.1 服务层整体架构

**核心服务分类：**

| 服务类别 | 关键模块 | 主要功能 |
|---------|---------|---------|
| **API 服务** | `api/` | Anthropic API 客户端、多供应商支持（Bedrock/Vertex/Foundry） |
| **MCP 协议** | `mcp/` | Model Context Protocol 客户端管理 |
| **LSP 服务** | `lsp/` | 语言服务器协议管理 |
| **上下文分析** | `compact/` | 对话压缩和摘要 |
| **记忆提取** | `extractMemories/` | 会话记忆自动提取 |
| **OAuth 认证** | `oauth/` | 用户身份验证 |
| **插件管理** | `plugins/` | 插件安装和生命周期管理 |

### 8.2 API 服务架构 (`api/`)

**核心功能 - 多供应商支持：**
- 直接 Anthropic API
- AWS Bedrock
- Google Vertex AI
- Azure Foundry

**关键文件：**
- **`client.ts`** - 客户端创建工厂，支持 PKCE OAuth 流程、自动 token 刷新
- **`claude.ts`** - 核心查询执行，支持流式和非流式输出

### 8.3 上下文压缩服务 (`compact/`)

**核心功能：**
- **完整对话压缩**: `compactConversation()` - 完整历史摘要
- **部分压缩**: `partialCompactConversation()` - 支持方向压缩
- **微压缩**: `microCompact.ts` - 轻量级优化
- **会话记忆压缩**: `sessionMemoryCompact.ts` - 记忆专项压缩

**压缩流程：**
1. 执行 `pre_compact` 钩子
2. 使用 `forked agent` 模式复用缓存（默认启用）
3. 生成摘要
4. 重建后压缩附件（最近访问文件、计划、已调用技能等）
5. 执行 `post_compact` 钩子

### 8.4 记忆提取服务 (`extractMemories/`)

**核心特性：**
- **自动记忆提取**: 在每个查询循环结束时触发
- **Forked Agent 模式**: 共享主对话的提示缓存
- **记忆目录**: `~/.claude/projects/<path>/memory/`
- **权限隔离**: 仅允许只读 Bash 命令、文件读操作、仅记忆目录内的写操作

### 8.5 MCP 服务 (`mcp/`)

**架构组件：**
- `MCPConnectionManager.tsx` - React Context 管理
- `useManageMCPConnections.ts` - 连接状态 Hook
- `client.ts` - 协议客户端
- `auth.ts` - 认证处理
- `oauthPort.ts` - OAuth 回调端口
- `channelPermissions.ts` - 通道权限控制

### 8.6 插件系统架构

**内置插件 (`builtinPlugins.ts`)：**
- ID 格式：`{name}@builtin`
- 用户可启用/禁用（持久化到设置）
- 可提供：技能、钩子、MCP 服务器

**插件安装管理器 (`PluginInstallationManager.ts`)：**
- 后台安装流程
- 计算市场差异 (`diffMarketplaces()`)
- 执行 `reconcileMarketplaces()`：安装缺失的、更新源变更的、跳过最新的

### 8.7 技能系统实现

**技能来源：**
| 来源 | 路径 |
|------|------|
| 策略设置 | `~/.claude/skills/` |
| 用户设置 | `$CLAUDE_CONFIG_HOME/skills/` |
| 项目设置 | `.claude/skills/` |
| 附加目录 | `--add-dir` 指定 |

**技能文件格式：**
```
skill-name/
  └── SKILL.md  (必须包含 Frontmatter)
```

**Frontmatter 字段：**
```yaml
name: 显示名称
description: 描述
when_to_use: 使用场景
allowed-tools: [bash, read, ...]
arguments: [arg1, arg2]
model: 特定模型
user-invocable: true/false
hooks: { pre, post, ... }
context: fork  # 执行上下文
agent: agent-id
effort: low|medium|high
paths: [ "src/**", "*.ts" ]  # 条件激活
```

**当前内置技能：**
1. `update-config` - 更新配置
2. `keybindings` - 键绑定
3. `verify` - 验证
4. `debug` - 调试
5. `lorem-ipsum` - 占位文本
6. `skillify` - 技能化
7. `remember` - 记忆
8. `simplify` - 简化
9. `batch` - 批处理
10. `stuck` - 脱困
11. `dream` (功能标志)
12. `loop` (AGENT_TRIGGERS)
13. `claude-api` (BUILDING_CLAUDE_APPS)

---

## 9. 整体架构总结

### 9.1 三层架构

```
┌─────────────────────────────────────────────────────────┐
│                   表现层 (Presentation)                  │
│  React + Ink UI、REPL 屏幕、Doctor 屏幕、Resume 屏幕    │
│  140+ 组件、150+ hooks、AppState 状态管理              │
└─────────────────────────┬───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│                   业务层 (Business)                      │
│  QueryEngine、工具系统、命令系统、Bridge 系统           │
│  工具调用循环、权限检查、消息流转                        │
└─────────────────────────┬───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│                   服务层 (Services)                      │
│  多供应商 API、MCP/LSP、OAuth、压缩、记忆提取、插件      │
└─────────────────────────────────────────────────────────┘
```

### 9.2 核心设计亮点

1. **快速路径启动**：cli.tsx 中的多个快速分支避免加载完整模块
2. **Forked Agent 模式**：压缩和记忆提取共享主对话缓存，降低成本
3. **多层权限检查**：工具池 → 工具 → 通用系统 → 钩子
4. **三种命令类型**：Prompt（模型驱动）、Local（本地执行）、LocalJSX（交互式 UI）
5. **极简状态管理**：自定义 Store，无 Redux/Zustand 依赖
6. **高效渲染**：`useSyncExternalStore` + 选择器模式
7. **模块化 Hooks**：150+ 个小型、专注的 hooks
8. **多供应商 API 抽象**：支持 Anthropic、Bedrock、Vertex、Foundry
9. **技能条件激活**：基于路径模式自动激活技能
10. **插件生命周期管理**：后台安装、更新和刷新
11. **闭包状态管理**：大量使用工厂函数模式而非类
12. **Dead Code Elimination**：使用 `feature('FLAG')` 进行条件编译

### 9.3 关键文件路径汇总

| 模块 | 文件路径 |
|------|---------|
| **启动入口** | `src/entrypoints/cli.tsx` |
| **主应用** | `src/main.tsx` |
| **QueryEngine** | `src/QueryEngine.ts` |
| **工具系统** | `src/Tool.ts`, `src/tools.ts` |
| **命令系统** | `src/commands.ts`, `src/commands/` |
| **Bridge 系统** | `src/bridge/` |
| **状态管理** | `src/state/` |
| **UI 组件** | `src/components/` |
| **屏幕** | `src/screens/` |
| **服务层** | `src/services/` |
| **插件系统** | `src/plugins/` |
| **技能系统** | `src/skills/` |

---

---

## 10. Rust 版本（Claw Code）对比分析

### 10.1 项目概述

Claw Code 是 Claude Code 的高性能 Rust 重写版本，项目位于 `rust/` 目录下的 Rust workspace。

| 属性 | 值 |
|--------|-----|
| **语言** | Rust (100% safe code) |
| **运行时** | 原生二进制 |
| **终端 UI** | 自定义 TUI (非 React/Ink) |
| **主要 crate 数量** | 9 个 |
| **核心入口** | `rusty-claude-cli` |

### 10.2 Rust Workspace Crate 架构

```
rust/crates/
├── rusty-claude-cli/      # 主二进制入口、REPL、CLI 解析
├── runtime/               # 核心对话循环、会话管理、配置、权限
├── api/                   # 多供应商 API 客户端（Anthropic、OpenAI、xAI、DashScope）
├── tools/                 # 内置工具实现
├── commands/              # Slash 命令定义
├── plugins/               # 插件元数据管理
├── telemetry/             # 会话追踪事件类型
├── mock-anthropic-service/ # 本地 Mock 服务
└── compat-harness/         # 兼容性测试框架
```

### 10.3 核心实现对比

| 功能模块 | TypeScript 版本 | Rust 版本 | 状态 |
|---------|----------------|----------|------|
| **核心引擎** | `QueryEngine.ts` (~46K 行) | `conversation.rs` | ✅ Rust 已实现 |
| **会话压缩** | `compact.ts` + `extractMemories/` | `compact.rs` + `summary_compression.rs` | ✅ Rust 已实现（启发式） |
| **Token 计数** | `cost-tracker.ts` + `tokens.ts` | `usage.rs` | ✅ Rust 已实现 |
| **思考模式** | `thinking.ts` | `api/src/types.rs` | ✅ Rust 已实现 |
| **插件系统** | `plugins/` + `PluginInstallationManager.ts` | `plugins/src/lib.rs` | ✅ Rust 已实现 |
| **技能系统** | `skills/` + `loadSkillsDir.ts` | `commands/src/lib.rs` | ✅ Rust 框架已实现 |
| **MCP** | `services/mcp/` | `runtime/src/mcp.rs` | ✅ Rust 已实现 |
| **LSP** | `services/lsp/` | `runtime/src/lsp.rs` | ✅ 注册表已实现 |
| **权限系统** | `types/permissions.ts` + `hooks/toolPermission/` | `permissions.rs` + `permission_enforcer.rs` | ✅ 基础已实现 |
| **Bridge 系统** | `bridge/` (8 个文件) | 未实现 | ❌ 缺失 |
| **UI 组件** | `components/` (140+ 组件) | 自定义 TUI | ⚠️ 简化实现 |

### 10.4 核心数据结构对比

#### TypeScript → Rust 对应关系：

| TypeScript 类型 | Rust 类型 | 位置 |
|---------------|----------|------|
| `QueryEngine` | `ConversationRuntime` | `runtime/src/conversation.rs` |
| `Message` | `ConversationMessage` | `runtime/src/session.rs` |
| `ContentBlock` | `ContentBlock` | `runtime/src/session.rs` |
| `Tool` | `ToolSpec` + `RuntimeToolDefinition` | `tools/src/lib.rs` |
| `ThinkingConfig` | `ThinkingConfig` | `api/src/types.rs` |
| `TokenUsage` | `TokenUsage` | `runtime/src/usage.rs` |

### 10.5 Rust 版本已实现的关键功能

✅ **会话压缩（启发式）：
- `compact_session()` - 基于规则的会话压缩
- `summary_compression.rs` - 摘要文本压缩
- 自动压缩集成到 `ConversationRuntime`
- `/compact` 命令可用

✅ **Token 计数和成本追踪：
- `TokenUsage` - input/output/cache 计数
- `ModelPricing` - haiku/sonnet/opus 定价
- `UsageTracker` - 会话累计追踪
- `/cost` 命令可用

✅ **插件系统：
- 插件元数据管理
- 插件工具注册
- `/plugins` 命令可用

✅ **技能系统框架：
- 技能加载和执行
- `/skills` 命令可用

✅ **MCP 完整实现：
- MCP 工具注册表
- MCP 工具调用桥接
- 完整的传输层实现

✅ **多供应商 API 支持：
- Anthropic Claude
- OpenAI-compatible (OpenAI、OpenRouter、Ollama 等
- xAI (Grok)
- DashScope (Qwen、Kimi)

✅ **完整的内置命令：
- `/compact` - 会话压缩
- `/cost` - 成本追踪
- `/doctor` - 诊断检查
- `/commit` - Git 提交
- `/plugins` - 插件管理
- `/skills` - 技能管理

### 10.6 Rust 版本设计亮点

1. **清晰的 crate 分离**：runtime、api、tools、commands、plugins 清晰分离
2. **trait 抽象**：`ApiClient`、`ToolExecutor` 等 trait 实现灵活扩展
3. **零 unsafe 代码**：全安全 Rust 实现
4. **JSONL 会话持久化**：追加式存储，支持旧 JSON 格式兼容
5. **配置层次加载**：用户配置 → 项目配置 → 本地覆盖
6. **Workspace 指纹**：FNV-1a 哈希路径指纹隔离不同工作区

### 10.7 依赖流向

```
rusty-claude-cli (main bin)
├── runtime (核心逻辑)
│   ├── api (类型定义)
│   ├── tools
│   ├── commands
│   └── plugins
├── api (客户端实现)
│   └── telemetry
└── tools
    └── runtime (文件操作)
```

---

## 附录

### A. 分析方法说明

本分析报告通过以下方式生成：
1. 子代理并行分析各个模块
2. 综合汇总各模块分析结果
3. 结构化整理成统一文档

### B. 版本信息

- **分析对象**: Claude Code TypeScript 版本源代码快照 + Rust 版本（Claw Code）
- **分析日期**: 2026-04-21
- **报告版本**: 2.0（添加 Rust 版本对比）

---

**报告结束**
