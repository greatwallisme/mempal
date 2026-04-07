---
name: rust
description: "Rust 编程综合技能。当用户要求编写、审查、调试、重构 Rust 代码时触发。也适用于 Cargo 项目初始化、依赖选型、架构设计、性能优化、unsafe 审查、测试策略、C/C++ 迁移等场景。覆盖标准库开发、异步编程、嵌入式、WebAssembly、CLI 工具、网络服务、密码学、FFI 等子领域。当用户提到 .rs 文件、Cargo.toml、crate、borrow checker、lifetime、trait、async/await、no_std、embedded-hal、tokio、axum、serde、clippy、miri 等关键词时，务必触发此 skill。"
---

# Rust 编程技能

## 何时使用此 Skill · 何时不使用

**使用**：涉及架构判断、API 设计、错误处理策略、依赖选型、unsafe 审查、跨模块重构、性能优化决策等需要工程品味的场景。

**不使用**：修 typo、加一行日志、格式化代码、解释一段代码在做什么、机械的语法修复。这类任务不需要启动概念锚点框架——直接做更高效。Skill 的价值在判断层面，不在操作层面。

---

## 设计哲学（一句话）

**Prompt 不是指令清单，是涌现的边界条件。** 本 skill 不用规则堆叠约束模型，而是用四层结构塑造输出空间：**理解 → 服从 → 释放 → 约束**。

---

## 工作流：四步顺序不可颠倒

### 第一步：理解（Situational Awareness）

在写任何代码之前，先建立对现状的理解。不要从真空里开始写 Rust——从这个项目已有的形状开始写。

```bash
# 项目结构
ls -la && cat Cargo.toml
# 已有的 crate 选型
grep -r "^[a-z_-]* = " Cargo.toml
# 近期代码风格
git log --oneline -20
# 已有的测试模式
find . -name "*.rs" -path "*/tests/*" | head -5
# 已有的错误类型定义
grep -rn "thiserror\|anyhow\|impl.*Error" src/ | head -20
```

**关键问题**（读代码时自己回答）：
- 这个项目已经选了 `thiserror` 还是 `anyhow`？不要跨过去用另一个。
- 异步用 `tokio` 还是 `async-std`？生态已定，别混。
- 错误处理是 `Result<T, MyError>` 风格还是 `anyhow::Result<T>` 风格？
- 已有代码的 `clone()` 密度、`unwrap()` 容忍度、生命周期标注的详细程度——这些都是默会的项目风格。

理解完成前不写代码。跳过这一步是最常见的失败模式。

### 第二步：服从（Submit to External Reality）

见下一节"服从层"。这是本 skill 和纯概念锚点最关键的区别。

### 第三步：释放（Activate Meta-Knowledge）

在理解和服从划定的空间内，用概念锚点激活正确的判断框架。见"释放层：核心概念锚点"一节。

### 第四步：约束（Deterministic Constraints）

跑确定性工具收口——格式、lint、测试。见"约束层"一节。

---

## 服从层：外部实在

Rust 生态提供了一套特别丰富的**外部实在**——比大多数语言都权威的、比模型判断更高的仲裁者。遇到冲突时，模型必须服从它们，不是和它们争论。

### 权威层级（从高到低）

1. **`cargo check`** — 这是 ground truth。如果它报错，你错了，不是编译器错了。不要绕过、不要 `allow`、不要加 `#[cfg]` 跳过——修。
2. **`cargo test`** — 已有测试是项目语义的固化。测试失败有两种可能：你的改动引入了问题（默认假设），或测试本身过时（需要明确论证）。不要为了让测试通过而修改测试。
3. **`cargo clippy`** — 社区积累的最佳实践的机械化版本。clippy 的警告有两种处理方式：修，或者加 `#[allow(...)]` 并在注释里解释为什么。没有"忽略"这个选项。
4. **`cargo +nightly miri test`**（涉及 unsafe 时）— 未定义行为的最终判决者。miri 报错 = 你有 UB，无例外。
5. **项目已有代码** — 在合理范围内的风格一致性优先于"更好的"改写。你来这里是加功能或修 bug，不是重构别人的品味。
6. **rustc 的错误消息** — rustc 的错误消息质量是 Rust 生态最大的无形资产之一，认真读它而不是急着猜。它经常直接告诉你怎么修。

### 冲突解决原则

- **clippy vs 你的直觉** → clippy 赢。clippy 错的情况存在但极罕见，先假设你错。
- **已有风格 vs 一般最佳实践** → 已有风格赢。除非明确被要求重构。
- **测试 vs 你的新代码** → 测试赢。先假设测试是对的。
- **miri vs 你觉得"应该没事"** → miri 赢。永远。
- **编译器 vs 你想用 `unsafe` 绕过** → 编译器赢。想用 unsafe 绕过编译器拒绝的事情是最危险的模式。

### 服从不是盲从

服从外部实在不等于不能挑战它。挑战的正确方式是：明确指出冲突点、给出修改它的论证（为什么 clippy 在这个场景错了、为什么这个测试反映的是旧需求、为什么这条已有风格应该被更新），然后让用户决定。**默认是服从，例外需要明确理由。**

---

## 释放层：核心概念锚点

在理解和服从划定的空间内，用以下锚点激活判断框架。**这些不是规则，是边界条件**——它们定义了"像谁一样思考"，中间的整合由模型完成。

### API 设计
参考 **dtolnay**（`serde`, `thiserror`, `anyhow`）和 **BurntSushi**（`ripgrep`, `regex`, `csv`）的风格。

- 共同点：API 表面积极小、命名精确、让错误用法在编译期被拦截
- 差异点：dtolnay 用 derive 宏降低使用者认知负担；BurntSushi 手工控制每层抽象，性能关键路径不妥协
- 选择：库代码偏 dtolnay，性能关键代码偏 BurntSushi

### 错误处理
参考 **BurntSushi** 的方法论：

- 让类型系统承载错误语义——每个变体都携带足够的上下文让调用者做程序化决策
- 让调用者决定处理策略——库不替调用者决定是 panic、log 还是 propagate
- 错误链必须保留——每层错误实现 `source()` 指向底层原因

选型（参考 **dtolnay** 的 thiserror/anyhow 设计分界）：
- 库代码 → `thiserror`（手写错误类型，类型精确）
- 应用代码 → `anyhow`（便利传播，减少样板）
- 同一项目不混用：库暴露 thiserror 类型，应用用 anyhow 包装

### 架构判断
参考 **Rich Hickey** 的 Simple vs Easy 区分：

- 先问"这是本质复杂性还是偶然复杂性"
- 优先追求本质简单（simple），而非表面方便（easy）
- 警惕 complecting——把不相关的关注点纠缠在一起

参考 **Sandi Metz** 的抽象决策：
- 宁可适度重复，也不要错误的抽象（Avoid Hasty Abstractions）
- 过早抽象比重复代码危害更大

### 类型系统运用
参考 **Niko Matsakis** 对 Rust 设计的阐述：

- 所有权和借用是验证设计意图的工具，不是限制
- 如果你在反复和 borrow checker 斗争，大概率是数据结构的所有权关系设计有问题
- trait 回答"这个类型能做什么"，而不是"这个类型是什么"

### 依赖选型
参考 **Dan McKinley** 的 Boring Technology 原则：

- 每个新依赖都是认知负债
- 问：这个 crate 解决的问题有多复杂？能用 50 行代码替代吗？维护状态如何？
- 反过来也成立：不要为了"零依赖"手写已有成熟实现的东西——那是另一种偶然复杂性

---

## 约束层：工具链命令

零歧义的事实配置，直接列出——不需要概念锚点。

```bash
# 服从层检查（每次改动必跑）
cargo check                        # 编译通过
cargo clippy -- -D warnings        # lint（警告视为错误）
cargo test                         # 测试通过
cargo fmt -- --check               # 格式一致

# 深度检查（涉及 unsafe 或性能时）
cargo +nightly miri test           # UB 检测
cargo bench                        # 基准测试（criterion）
cargo flamegraph                   # 性能热点

# 依赖审查
cargo audit                        # 安全漏洞
cargo deny check                   # 许可证和依赖策略
cargo outdated                     # 过时依赖
cargo tree                         # 依赖树

# 文档
cargo doc --open                   # 生成并打开文档
cargo doc --no-deps --document-private-items
```

**原则**：格式和 lint 交给确定性工具，不要用 prompt 去做 `rustfmt`/`clippy` 能做的事。把 prompt 的表达空间留给需要判断的领域。

---

## Review 检查点：外部实在优先，主观判断兜底

审查 Rust 代码时，**先跑外部实在检查，后做主观判断**。顺序不能颠倒。

### 第一层：外部实在（必须全部通过）

```bash
cargo check                                      # 1. 编译通过
cargo clippy -- -W clippy::pedantic              # 2. 包括 pedantic 等级的 lint
cargo test                                       # 3. 所有测试通过
cargo +nightly miri test                         # 4. 涉及 unsafe 时必跑
cargo fmt -- --check                             # 5. 格式一致
```

每一条 clippy 警告要么修，要么显式 `#[allow(...)]` 加注释说明理由。没有"忽略"这个选项。

### 第二层：项目一致性（对照已有代码）

- 新增的错误类型风格和已有的一致吗？
- 新增的模块命名约定和已有的一致吗？
- 新增的 public API 的文档注释详细程度和已有的一致吗？
- 新增的测试组织方式和已有的一致吗（`#[cfg(test)] mod tests` vs `tests/` 目录）？

不一致不一定是错——但必须有理由。

### 第三层：主观判断（前两层都通过后用）

这一层是概念锚点真正起作用的地方。前两层通过之后，再用主观判断评估涌现质量：

- **类型是否承载了足够的语义？** 参考 BurntSushi 的错误类型设计——类型是领域知识的编码。但同时参考 Sandi Metz——如果 newtype 只是为"强类型"本身而没有防止实际误用，那是错误的抽象。
- **所有权关系是否反映真实的数据流？** 参考 Niko Matsakis——如果需要 `Arc<Mutex<T>>` 才能编译，先停下来画一画数据在组件间的流向。很可能有更简单的所有权模型没被看见。
- **错误路径和正常路径一样被认真设计了吗？** 每个 `?` 传播的错误是否携带了足够的上下文？调用者拿到这个错误后能做什么？如果答案只有"打印"，错误类型可能需要补充信息。
- **依赖值得引入吗？** 参考 Dan McKinley——每个新依赖都是认知负债。

---

## 领域子技能

根据项目类型，读对应的参考文件获取领域专属锚点。

**注意**：不同领域的锚点密度差异很大。主流领域（async、web、CLI）有充分的概念锚点覆盖，可以纯靠人名和项目名激活。小众框架（Makepad、Slint 等）在训练数据中覆盖较薄，概念锚点会失效——此时必须退回到源码样本作为外部实在。参考文件会标注采用哪种策略。

| 领域 | 参考文件 | 何时阅读 |
|------|---------|---------|
| 异步编程 | `references/async.md` | tokio, async/await, Future, Stream |
| 嵌入式 | `references/embedded.md` | no_std, MCU, embedded-hal, RTIC, Embassy |
| unsafe | `references/unsafe.md` | unsafe block, FFI, raw pointer, 内存布局 |
| Web 服务 | `references/web.md` | HTTP server, API, middleware, axum |
| CLI 工具 | `references/cli.md` | 命令行工具、参数解析、TUI |
| 密码学与安全 | `references/crypto.md` | 加密、TLS、安全审计 |
| 测试策略 | `references/testing.md` | 单元测试、快照测试、属性测试、基准 |
| C/C++ 迁移 | `references/migration.md` | c2rust, cxx, bindgen, FFI, 代码迁移 |
| GUI (Makepad) | `references/gui-makepad.md` | Makepad, live_design!, 跨平台 GUI, shader UI |

---

## 反模式

Rust 特有的已知失败模式。遇到时停下来，不要硬推。

**滥用 `.clone()` 回避所有权问题** — 当 borrow checker 报错时，加 `.clone()` 能编译但常掩盖设计问题。先想清楚谁应该拥有这个数据。

**过度使用 `Box<dyn Trait>` 代替泛型** — 动态分发有运行时开销。编译期已知具体类型时用 `impl Trait` 或泛型参数。

**在库中 `println!` 做日志** — 库应该用 `log` 或 `tracing` 门面，让调用者决定日志输出方式。

**混淆 `String` 和 `&str` 的 API 设计** — 函数参数优先接受 `&str`（或 `impl AsRef<str>`），只在需要拥有所有权时用 `String`。

**在 async 代码中持有锁跨 `.await`** — 会导致死锁或性能退化。缩小锁粒度或用 `tokio::sync::Mutex`。

**用 `unsafe` 绕过编译器的"烦人"拒绝** — 这是所有 unsafe 误用中最危险的一种。编译器拒绝你通常是有理由的。先彻底理解它为什么拒绝，再考虑 unsafe 是否真的必要。

**和 clippy 争论而不服从** — 99% 的情况下 clippy 是对的。挑战 clippy 的正确姿势是明确论证为什么这个场景是那 1%，而不是直接 `#[allow]` 掉。
