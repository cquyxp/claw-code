# Claude Code 完整功能差距清单

> 基于 TypeScript 版本源代码分析，对比 Rust 版本当前实现
>
> 分析日期：2026-04-21

---

## 目录

1. [🔴 核心功能差距](#-核心功能差距)
2. [🟡 重要功能差距](#-重要功能差距)
3. [🟢 用户体验增强](#-用户体验增强)
4. [🔵 服务层功能](#-服务层功能)
5. [📊 总结](#-总结)

---

## 🔴 核心功能差距

### 1. Bridge 系统（IDE 集成）

**功能描述**：实现 Claude Code CLI 与 claude.ai Web IDE 之间的双向通信桥接

**缺失功能**：
- [ ] WebSocket/SSE + REST API 与 claude.ai 通信
- [ ] JWT 令牌管理和自动刷新
- [ ] Work Secret 机制
- [ ] 多会话管理（single-session/worktree/same-dir）
- [ ] 会话超时看门狗（24 小时）
- [ ] 心跳续约机制
- [ ] 远程会话状态同步

**关键文件**（TypeScript 版本）：
- `src/bridge/bridgeMain.ts` - 桥接主循环
- `src/bridge/bridgeMessaging.ts` - 消息协议
- `src/bridge/jwtUtils.ts` - JWT 认证
- `src/bridge/sessionRunner.ts` - 会话执行管理

---

### 2. 完整的 UI 组件系统

**功能描述**：基于 React + Ink 的完整终端 UI 组件库

**缺失功能**：
- [ ] React + Ink 完整组件库（140+ 组件）
- [ ] 主题系统（ThemeProvider）
- [ ] PromptInput 模块（18 个文件）
  - [ ] 历史搜索（HistorySearchInput）
  - [ ] 自动补全（Typeahead）
  - [ ] Vim 模式输入
  - [ ] 语音模式指示器（VoiceIndicator）
  - [ ] 通知系统（Notifications）
- [ ] 虚拟滚动消息列表（VirtualMessageList）
- [ ] 权限请求对话框（PermissionRequest）
- [ ] 消息选择器（MessageSelector）

**关键文件**（TypeScript 版本）：
- `src/components/design-system/` - 主题系统
- `src/components/PromptInput/` - 提示输入模块
- `src/screens/REPL.tsx` - 主交互界面

---

### 3. 思考模式（Thinking Mode）

**功能描述**：支持模型的 extended thinking 模式

**状态**：✅ **Rust 版本已实现**

**Rust 实现位置**：
- `api/src/types.rs` - `ThinkingConfig` 类型定义
- `api/src/providers/anthropic.rs` - API 调用支持
- 支持 `thinking` 类型和 `budget_tokens` 配置

**关键文件**（TypeScript 版本）：
- `src/utils/thinking.ts` - 思考模式配置
- `src/services/api/claude.ts` - 思考模式 API 调用

---

### 4. 会话压缩

**功能描述**：自动压缩会话历史

**状态**：✅ **Rust 版本已实现**（启发式压缩）

**Rust 实现位置**：
- `runtime/src/compact.rs` - `compact_session()` 启发式压缩
- `runtime/src/summary_compression.rs` - 摘要压缩
- `runtime/src/conversation.rs` - 自动压缩集成
- `/compact` 命令可用

**待定**：
- [ ] LLM 驱动的会话压缩（待定）

**关键文件**（TypeScript 版本）：
- `src/services/compact/compact.ts` - 对话压缩
- `src/services/extractMemories/extractMemories.ts` - 记忆提取

---

### 5. 推测执行（Speculative Execution）

**功能描述**：在用户确认前预执行工具调用，提升响应速度

**缺失功能**：
- [ ] 推测状态管理（`speculation: SpeculationState`）
- [ ] 推测会话时间统计（`speculationSessionTimeSavedMs`）
- [ ] 推测执行的 UI 反馈
- [ ] 推测执行的确认/回滚机制

**关键文件**（TypeScript 版本）：
- `src/state/AppStateStore.ts` - 推测状态定义
- `src/screens/REPL.tsx` - 推测执行 UI

---

## 🟡 重要功能差距

### 6. Token 计数和成本追踪

**状态**：✅ **Rust 版本已实现**

**Rust 实现位置**：
- `runtime/src/usage.rs` - 完整的 token 计数和成本估算
- `ModelPricing` 支持 haiku/sonnet/opus 定价
- `UsageTracker` 追踪会话累计使用量
- `/cost` 命令可用

---

### 7. 高级权限系统

**功能描述**：完整的权限检查和自动决策系统

**Rust 状态**：✅ **基础权限系统已实现**

**Rust 实现位置**：
- `runtime/src/permissions.rs` - `PermissionMode` 和 `PermissionPolicy`
- `runtime/src/permission_enforcer.rs` - 权限执行器

**缺失功能**：
- [ ] 权限分类器（`auto` 模式）
- [ ] 权限模式：`bubble`、`plan` 等
- [ ] 权限钩子
  - [ ] `PreToolUse` 钩子
  - [ ] `PostToolUse` 钩子
  - [ ] `PostToolUseFailure` 钩子
- [ ] 自动记忆权限隔离（`createAutoMemCanUseTool()`）
- [ ] 权限规则匹配（`matchWildcardPattern`）

**关键文件**（TypeScript 版本）：
- `src/types/permissions.ts` - 权限类型
- `src/hooks/toolPermission/` - 权限钩子

---

### 8. Bash 验证

**功能描述**：Bash 命令安全验证

**Rust 状态**：✅ **基础实现已存在**

**Rust 实现位置**：
- `tools/src/bash.rs` - Bash 工具实现

**可能缺失功能**（需进一步验证）：
- [ ] `readOnlyValidation` - 只读验证
- [ ] `destructiveCommandWarning` - 破坏性命令警告
- [ ] `modeValidation` - 模式验证
- [ ] `sedValidation` - sed 编辑解析
- [ ] `pathValidation` - 路径验证
- [ ] `commandSemantics` - 命令语义分析

**关键文件**（TypeScript 版本）：
- `src/tools/Bash/bashPermissions.ts` - Bash 权限
- `src/tools/Bash/bashSecurity.ts` - Bash 安全
- `src/tools/Bash/commandSemantics.ts` - 命令语义

---

### 9. 插件系统

**状态**：✅ **Rust 版本已实现**

**Rust 实现位置**：
- `plugins/src/lib.rs` - 完整插件系统
- `commands/src/lib.rs` - `/plugins` 命令
- 支持插件元数据、生命周期管理

**可能缺失功能**（需进一步验证）：
- [ ] 插件安装管理器（后台安装流程）
- [ ] 市场差异计算（`diffMarketplaces()`）
- [ ] 插件提供技能、钩子、MCP 服务器

**关键文件**（TypeScript 版本）：
- `src/services/plugins/PluginInstallationManager.ts` - 插件安装管理
- `src/plugins/builtinPlugins.ts` - 内置插件

---

### 10. 技能系统

**状态**：✅ **Rust 版本已实现**

**Rust 实现位置**：
- `commands/src/lib.rs` - `/skills` 命令
- 技能加载和执行框架

**可能缺失功能**（需进一步验证）：
- [ ] 条件技能激活（`paths` 字段）
- [ ] 技能参数替换（`${argName}`、`${CLAUDE_SKILL_DIR}` 等）
- [ ] 13+ 内置技能完整实现

**关键文件**（TypeScript 版本）：
- `src/skills/loadSkillsDir.ts` - 技能加载
- `src/skills/bundled/index.ts` - 内置技能

---

### 11. 多供应商 API 支持

**状态**：✅ **Rust 版本已实现**

**Rust 实现位置**：
- `api/src/providers/` - 多提供商支持
  - `anthropic.rs` - Anthropic Claude
  - `openai_compat.rs` - OpenAI、OpenRouter、xAI、DashScope
- 支持模型：Claude、GPT、Qwen、Kimi、Grok 等

**可能缺失功能**（需进一步验证）：
- [ ] AWS Bedrock 完整支持
- [ ] Google Vertex AI 完整支持
- [ ] Azure Foundry 完整支持
- [ ] PKCE OAuth 流程
- [ ] 自动 token 刷新

**关键文件**（TypeScript 版本）：
- `src/services/api/client.ts` - 客户端工厂
- `src/services/api/claude.ts` - 核心查询

---

### 12. MCP（Model Context Protocol）

**状态**：✅ **Rust 版本已实现**

**Rust 实现位置**：
- `runtime/src/mcp.rs` - MCP 注册表和桥接
- 完整的 MCP 工具注册和调用机制

**可能缺失功能**（需进一步验证）：
- [ ] MCP OAuth 端口
- [ ] MCP 通道权限控制
- [ ] MCP 认证处理

**关键文件**（TypeScript 版本）：
- `src/services/mcp/MCPConnectionManager.tsx` - 连接管理
- `src/services/mcp/client.ts` - MCP 客户端

---

### 13. LSP 客户端

**Rust 状态**：✅ **注册表/分派已实现**

**Rust 实现位置**：
- `runtime/src/lsp.rs` - LSP 注册表

**可能缺失功能**（需进一步验证）：
- [ ] LSP 服务器进程编排
- [ ] 完整的 LSP 方法实现
- [ ] LSP 请求发送机制

**关键文件**（TypeScript 版本）：
- `src/services/lsp/LSPServerManager.ts` - LSP 管理器

---

### 14. 团队和多代理系统

**功能描述**：多代理协作和团队管理

**缺失功能**：
- [ ] 代理内存管理（`agentMemory`）
- [ ] Fork 子代理（`forkSubagent`）
- [ ] 代理定义加载（`loadAgentsDir`）
- [ ] 团队协作（`SendMessageTool`）
- [ ] 代理注册表（`agentNameRegistry`）
- [ ] 前台任务（`foregroundedTaskId`）
- [ ] 查看代理任务（`viewingAgentTaskId`）

**关键文件**（TypeScript 版本）：
- `src/services/agents/` - 代理服务
- `src/tools/AgentTool.ts` - 代理工具

---

### 15. Cron 定时任务

**Rust 状态**：✅ **注册表已实现**

**Rust 实现位置**：
- `runtime/src/cron.rs` - Cron 注册表

**可能缺失功能**（需进一步验证）：
- [ ] 后台调度器
- [ ] 持久化定时任务
- [ ] 定时任务执行引擎
- [ ] Cron 表达式解析

**关键文件**（TypeScript 版本）：
- `src/tools/ScheduleCronTool.ts` - Cron 工具
- `src/services/cron/` - Cron 服务

---

### 16. Voice 语音模式

**功能描述**：语音输入和语音模式

**缺失功能**：
- [ ] 语音输入处理
- [ ] 语音指示器 UI（`VoiceIndicator`）
- [ ] 语音模式配置
- [ ] 语音转文字集成

**关键文件**（TypeScript 版本）：
- `src/voice/` - 语音模块
- `src/components/PromptInput/VoiceIndicator.tsx` - 语音指示器

---

## 🟢 用户体验增强

### 17. 通知系统

**功能描述**：完整的用户通知系统

**缺失功能**：
- [ ] 启动通知（`useStartupNotification`）
- [ ] 快速模式通知（`useFastModeNotification`）
- [ ] 插件安装状态通知（`usePluginInstallationStatus`）
- [ ] 设置错误通知（`useSettingsErrors`）
- [ ] Chrome 扩展通知（`useChromeExtensionNotification`）

**关键文件**（TypeScript 版本）：
- `src/hooks/useStartupNotification.ts`
- `src/hooks/useFastModeNotification.ts`
- `src/components/PromptInput/Notifications.tsx`

---

### 18. 反馈调查

**功能描述**：用户反馈收集

**缺失功能**：
- [ ] 技能改进调查（`SkillImprovementSurvey`）
- [ ] 用户反馈收集
- [ ] 反馈提交机制

**关键文件**（TypeScript 版本）：
- `src/components/FeedbackSurvey/` - 反馈调查

---

### 19. Logo 和动画

**功能描述**：品牌标识和动画效果

**缺失功能**：
- [ ] LogoV2 动画组件
- [ ] Buddy 精灵系统（`buddy/`）
- [ ] 同伴通知（`useBuddyNotification`）
- [ ] 精灵动画（`sprites.ts`）

**关键文件**（TypeScript 版本）：
- `src/components/LogoV2/` - Logo 动画
- `src/buddy/` - Buddy 精灵系统

---

### 20. Vim 模式

**功能描述**：完整的 Vim 编辑模式

**缺失功能**：
- [ ] Vim 输入处理（`useVimInput`）
- [ ] Vim 键绑定
- [ ] Vim 模式切换
- [ ] Vim 命令模式

**关键文件**（TypeScript 版本）：
- `src/vim/` - Vim 模式
- `src/hooks/useVimInput.ts` - Vim 输入 Hook

---

### 21. 键绑定系统

**功能描述**：可配置的键盘快捷键

**缺失功能**：
- [ ] 可配置键绑定
- [ ] 和弦绑定（Chord bindings）
- [ ] 键绑定帮助
- [ ] 键绑定配置 UI

**关键文件**（TypeScript 版本）：
- `src/keybindings/` - 键绑定配置

---

## 🔵 服务层功能

### 22. 分析服务

**功能描述**：功能标志、分析、遥测

**缺失功能**：
- [ ] GrowthBook 功能标志和分析
- [ ] OpenTelemetry + gRPC 遥测
- [ ] 组织策略限制（`policyLimits`）
- [ ] 远程管理设置（`remoteManagedSettings`）

**关键文件**（TypeScript 版本）：
- `src/services/analytics/` - 分析服务
- `src/services/policyLimits/` - 策略限制

---

### 23. 服务器模式

**功能描述**：作为服务器运行，提供 API

**缺失功能**：
- [ ] 完整的服务器实现
- [ ] HTTP API
- [ ] 远程会话管理
- [ ] 认证和授权

**关键文件**（TypeScript 版本）：
- `src/server/` - 服务器模式

---

### 24. 桌面和移动端集成

**功能描述**：与桌面和移动应用的切换和集成

**缺失功能**：
- [ ] 桌面应用切换（`/desktop`）
- [ ] 移动端切换（`/mobile`）
- [ ] IDE 集成（`useIDEIntegration`）
- [ ] IDE 连接状态（`useIdeConnectionStatus`）
- [ ] IDE 选区（`useIdeSelection`）

**关键文件**（TypeScript 版本）：
- `src/hooks/useIDEIntegration.ts`
- `src/commands/desktop/index.ts`
- `src/commands/mobile/index.ts`

---

## 📊 总结

### 功能差距统计

| 类别 | 功能数量 | 优先级 |
|------|---------|--------|
| 🔴 核心功能 | 5 | 最高 |
| 🟡 重要功能 | 11 | 高 |
| 🟢 用户体验增强 | 5 | 中 |
| 🔵 服务层功能 | 3 | 中 |
| **总计** | **24** | - |

### 主要模块统计

- **核心文件数**（TypeScript）：~1,900 个
- **代码行数**（TypeScript）：512,000+ 行
- **组件数**：140+ 个
- **Hooks 数**：150+ 个
- **内置工具数**：40+ 个
- **内置命令数**：80+ 个
- **内置技能数**：13+ 个

### Rust 版本当前状态

✅ **已完整实现**：
- 核心 QueryEngine 引擎（ConversationRuntime）
- Token 计数和成本追踪（`usage.rs`）
- 会话压缩（启发式实现，`compact.rs`）
- 基础工具（Bash、Read、Write、Edit、Glob、Grep）
- 基础命令系统（80+ 命令）
- `/compact`、`/cost`、`/doctor`、`/commit`、`/plugins`、`/skills` 命令
- 思考模式 API 支持
- 基本权限系统（`permissions.rs`）
- 会话持久化（JSONL 格式）
- 插件系统（`plugins` crate）
- 技能系统框架
- MCP 完整实现（`mcp.rs`）
- 多供应商 API 支持（Anthropic、OpenAI、xAI、DashScope）
- LSP 注册表
- Task/Team/Cron 注册表

⏳ **部分实现**：
- Bash 验证（基础实现存在，需验证完整的 18 个子模块）
- MCP 生命周期（基础实现存在，需验证 OAuth/权限控制）
- LSP 客户端（注册表存在，需验证进程编排）
- Cron 定时任务（注册表存在，需验证后台调度器）

❌ **未实现**：
- Bridge 系统（IDE 集成）
- 完整 React + Ink UI 组件系统
- 推测执行
- LLM 驱动的会话压缩（待定）
- 团队和多代理系统
- Voice 语音模式
- 通知系统
- Vim 模式
- 键绑定系统
- 分析服务（GrowthBook、OpenTelemetry）
- 服务器模式
- 桌面和移动端集成

---

## 实施建议

### 阶段 1：核心功能（最高优先级）
1. Bridge 系统基础（IDE 集成）
2. LLM 驱动的会话压缩（待定）
3. 完整 Bash 验证（18 个子模块）

### 阶段 2：重要功能（高优先级）
1. MCP OAuth 和权限控制
2. LSP 服务器进程编排
3. Cron 后台调度器
4. 团队和多代理系统

### 阶段 3：用户体验（中优先级）
1. 通知系统
2. Vim 模式
3. 键绑定系统
4. Voice 语音模式

---

**文档结束**
