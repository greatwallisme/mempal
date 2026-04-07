# 测试策略

## 概念锚点

### 测试哲学
参考 **Matklad (Aleksey Kladov)**（rust-analyzer 作者）的测试方法论：

- 测试是设计工具，不只是质量工具——难以测试的代码通常是设计有问题的代码
- 偏好集成测试而非碎片化的单元测试——集成测试捕获真实的接口契约
- 测试应该 fail fast 且诊断信息清晰——一个失败的测试应该告诉你哪里出了问题，而不是需要你去调查
- 测试代码的可读性和产品代码同等重要——测试也是文档

他的 "How to Test" 博客系列是 Rust 社区事实上的测试风格指南。

### 快照测试
参考 **Armin Ronacher** 的 `insta`：

- 对任何涉及复杂输出结构（格式化文本、AST、序列化结果、错误消息）的测试，优先用快照而非手写 `assert_eq!`
- 快照让测试意图和实现细节分离——改了输出格式时，`cargo insta review` 让你看到 diff 再决定接受
- 适合的场景：CLI 输出、错误消息、代码生成器、pretty-printer、配置渲染

### 属性测试
参考 **proptest** 和 **quickcheck** 的方法论：

- 不是替代单元测试，是补充——用于数据处理、解析、序列化/反序列化对偶、数学运算
- 好的属性测试找不变量（invariants）：解析后再序列化应该得到原始输入、排序后的列表元素集合不变、往 Vec 里 push 后 pop 应该得到原元素
- 属性测试发现 bug 后，`proptest` 的 shrinking 会给你一个最小复现案例——这是它最大的价值

### 基准测试
参考 **criterion** 的统计严谨性：

- 不要用 `std::time::Instant` 手工计时——你测出的噪声会比信号大
- criterion 做统计显著性检验、自动 warmup、异常值检测
- 性能改进必须有 criterion 报告作为证据——"感觉快了"不算

### 测试运行器
参考 **cargo-nextest**（nextest.rs）：

- 比 `cargo test` 显著更快（并行策略更好、编译缓存更智能）
- 更清晰的失败输出——一屏看到所有失败测试而不是刷屏
- 支持 retry、超时、分片——CI 场景的标配
- 在有规模的项目上，`cargo nextest run` 应该替代 `cargo test` 作为默认命令

## 工具链

```bash
# 运行测试
cargo test                          # 默认运行器
cargo nextest run                   # 更快的运行器
cargo test --doc                    # 仅运行文档测试
cargo test -- --nocapture           # 显示 println!/dbg! 输出
cargo test -- --test-threads=1      # 串行运行（用于调试竞争）

# 快照测试（insta）
cargo insta test                    # 运行并生成快照
cargo insta review                  # 交互式 review 待确认的快照
cargo insta accept                  # 接受所有待确认快照

# 覆盖率
cargo llvm-cov                      # 基于 LLVM source-based coverage
cargo llvm-cov --html               # HTML 报告

# 基准测试
cargo bench                         # 运行 criterion 基准
cargo criterion                     # 如果安装了 cargo-criterion，更丰富的输出
```

## 典型依赖组合

```toml
[dev-dependencies]
# 快照测试
insta = { version = "1", features = ["yaml", "json"] }

# 属性测试
proptest = "1"

# 基准测试
criterion = { version = "0.5", features = ["html_reports"] }

# 测试辅助
pretty_assertions = "1"             # 更好的 assert_eq! diff 输出
rstest = "0.23"                     # 参数化测试和 fixture
tempfile = "3"                      # 临时文件/目录（替代手工 /tmp 路径）
mockall = "0.13"                    # mock 生成（仅在必要时——通常集成测试更好）

[[bench]]
name = "my_bench"
harness = false                     # criterion 需要关闭默认 harness
```

## 测试组织的判断

**何时用单元测试（`#[cfg(test)] mod tests`）**
- 纯函数的边界情况
- 私有实现细节的正确性
- 单个模块内部的逻辑

**何时用集成测试（`tests/` 目录）**
- 测试公共 API 的契约
- 多模块协作的正确性
- 和外部资源（文件系统、网络、数据库）的交互

**何时用文档测试（`///` 注释中的代码块）**
- API 的使用示例——同时起文档和测试的作用
- 防止文档里的例子随代码漂移

**何时用快照测试**
- 输出结构复杂且手写断言会淹没意图
- 输出可能随时间演化但需要 diff review 的场景

**何时用属性测试**
- 存在数学不变量的代码
- 序列化/反序列化、解析/格式化的对偶关系
- 数据结构操作（插入后查找、排序后有序）

**何时用基准测试**
- 对性能有明确要求的代码路径
- 验证优化是否有效（必须在优化前后都跑）

## 反模式

**测试私有实现而非公共契约** — 测试应该通过公共 API 验证行为，这样重构实现时测试不需要改。只有在纯函数或 critical 私有逻辑时才测私有代码。

**用 mock 代替集成测试** — 过度 mock 的测试给你虚假的安全感——组件独立工作不代表它们一起工作。Matklad 的原则：优先集成测试，mock 是退路。

**测试名字只说"test_X"** — 好的测试名字描述行为：`test_parse_rejects_trailing_comma` 而不是 `test_parse_1`。测试失败时你应该能从名字看出出了什么问题。

**忽略 flaky 测试** — 一个 flaky 测试是一个 bug 信号，不是"运气不好"。要么找到原因修掉，要么明确标记 `#[ignore]` 并记录为什么。绝对不要把"重试三次直到通过"当作解决方案。

**基准测试没有 baseline 对比** — 性能数字单独看没有意义，必须和改动前的数字对比。criterion 会自动做这件事，但你必须在优化前先跑一次 baseline。
