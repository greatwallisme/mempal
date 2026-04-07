# C/C++ 到 Rust 迁移

## 两个根本不同的场景

C→Rust 和 C++→Rust 看似都是"迁移"，但工具链、策略、难度完全不同。先分清你在哪个场景。

### C → Rust：有成熟的自动化路径

**第一锚点：c2rust（Immunant / Galois）**

c2rust 是 C-to-Rust 迁移的事实标准工具。它做的事情很明确：把 C99 代码逐行翻译成**语义等价的 unsafe Rust**。翻译后的代码能编译、能通过原有测试，但充满 raw pointer 和 unsafe block——本质上是"穿着 Rust 外衣的 C"。

这不是缺陷，这是设计。c2rust 的哲学是：先保证语义正确，再逐步提升安全性。参考 NDSS 2025 的用户研究发现：成功的人工翻译都是从低层抽象**语义提升**到 Rust 惯用模式，而不是试图保留原始的数据流结构。

**迁移流水线（c2rust 路径）：**

```
C 源码
  ↓ compile_commands.json（由 CMake/Bear 生成）
  ↓ c2rust transpile
unsafe Rust（语义等价，充满 raw pointer）
  ↓ 逐模块人工/AI 辅助重构
  ↓ 每一步用原有测试验证语义不变
idiomatic safe Rust
```

关键工具链：
```bash
# 生成编译数据库
cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON ..
# 或用 Bear 拦截任意构建系统
bear -- make

# 执行转译
cargo install c2rust --locked
c2rust transpile compile_commands.json --binary my_program

# 转译后验证
cargo build    # 确认编译通过
cargo test     # 确认原有测试通过
```

**第二锚点：NDSS 2025 用户研究的核心发现**

该研究对比了自动工具和人工翻译，得出了模型应该内化的关键原则：

- **语义提升，不是语法搬运**：成功的翻译把 C 的 output parameter pattern（`int func(int *result)`）提升为 Rust 的 `Result<i32, Error>` 返回值，而不是保留 `*mut i32` 指针
- **重新设计所有权关系**：C 的指针别名模式在 Rust 中大量违反借用规则。必须重新设计谁拥有数据、谁借用、生命周期边界在哪里
- **分阶段替换 unsafe**：每次只消除一类 unsafe 用法（如 raw pointer → reference），消除后跑测试，确认语义不变再继续下一类

**LLM 辅助迁移（2024-2025 研究前沿）：**

最新研究（RustFlow、Rustine、C2SaferRust 等）表明 LLM 在以下步骤上有独特优势：

- 识别 C 的错误处理惯用法（设置 errno + goto cleanup）并翻译为 `Result` + `?` 操作符——这是静态工具难以做到的语义级模式识别
- 将 C 的 `void*` 多态翻译为 Rust 泛型或 trait object
- 将 libc 函数调用替换为 Rust 标准库等价物

但 LLM 的弱点同样明显：对复杂的指针别名关系和跨函数的生命周期推理仍然不可靠。这些需要人工审查。

---

### C++ → Rust：没有自动化银弹

C++ 到 Rust 没有等价于 c2rust 的自动转译工具。原因是 C++ 的语义复杂度——模板、RAII、异常、多重继承、运算符重载——远超任何自动翻译器的能力。

这个方向的核心策略是**渐进式互操作**，而非一步到位的翻译。

**第一锚点：cxx（dtolnay）**

cxx 是 C++↔Rust 安全互操作的核心工具。它的设计哲学和 bindgen 根本不同：

- **bindgen** 生成 C 风格的 `extern "C"` 绑定——所有调用都是 unsafe，类型映射是低层的
- **cxx** 用一个 bridge 模块声明双向接口——编译期验证签名匹配，支持高层类型（`String` ↔ `std::string`，`Vec` ↔ `std::vector`，`Box` ↔ `unique_ptr`），生成的 FFI 调用是零开销的

cxx bridge 的关键设计：
```rust
#[cxx::bridge]
mod ffi {
    // 两端共享的类型——单一事实源
    struct Metadata {
        size: usize,
        tags: Vec<String>,
    }

    // Rust 侧定义，暴露给 C++
    extern "Rust" {
        type MyBuffer;
        fn process(buf: &mut MyBuffer) -> Result<()>;
    }

    // C++ 侧定义，暴露给 Rust
    unsafe extern "C++" {
        include!("mylib/include/client.h");
        type Client;
        fn new_client() -> UniquePtr<Client>;
        fn send(&self, data: &[u8]) -> Result<usize>;
    }
}
```

**第二锚点：autocxx（Adrian Taylor / Google Chrome）**

autocxx 构建在 cxx 之上，试图自动从 C++ 头文件生成 bridge——类似 bindgen 的自动化程度但保留 cxx 的安全保证。主要用于 Google Chrome 的 Rust 集成实验。尚不如 cxx 成熟，但代表了未来方向。

**第三锚点：Android / Rust for Linux 的渐进式策略**

Google 在 Android 中引入 Rust 的策略不是翻译已有 C++，而是：
- 新模块用 Rust 写
- 通过 FFI 边界和已有 C++ 代码互操作
- 逐步缩小 C++ 的范围

Linux 内核（Miguel Ojeda 主导的 Rust for Linux 项目）采用类似策略：Rust 模块通过明确定义的内核 API 边界与 C 代码交互，不试图翻译已有的 C 内核代码。

---

## 工具生态速查

| 工具 | 方向 | 用途 | 安全性 |
|------|------|------|--------|
| **c2rust** | C → Rust | 自动转译，语义等价 | 生成 unsafe Rust |
| **bindgen** | C/C++ → Rust | 从头文件生成 FFI 绑定 | 所有调用 unsafe |
| **cbindgen** | Rust → C/C++ | 从 Rust 生成 C/C++ 头文件 | - |
| **cxx** | C++ ↔ Rust | 安全双向互操作 bridge | 编译期验证，大部分 safe |
| **autocxx** | C++ → Rust | 自动生成 cxx bridge | 继承 cxx 安全性 |

## 迁移决策树

```
你有一个 C 代码库想迁到 Rust？
├── 是 C（不是 C++）
│   ├── 代码库 < 10K 行 → 考虑直接用 LLM 辅助重写
│   └── 代码库 > 10K 行 → c2rust 转译 + 逐模块重构
└── 是 C++
    ├── 需要完全替换 C++ → 长期战略：新代码用 Rust + cxx 互操作 + 逐步缩小 C++ 范围
    └── 只需要从 Rust 调用 C++ 库 → cxx bridge（首选）或 bindgen（退路）
```

## 概念锚点 Prompt 模板

**C→Rust 迁移场景：**
```
你正在帮助将 C 代码迁移到惯用 Rust。

迁移哲学：
- 参考 c2rust 的两阶段方法：先语义等价转译，再逐步提升安全性
- 重构策略参考 NDSS 2025 用户研究的发现：语义提升而非语法搬运——
  把 output parameter 变成 Result 返回值，把 malloc/free 变成 Vec 或 Box，
  把 goto cleanup 变成 ? 操作符
- 每一步重构后用原有测试验证语义不变
- unsafe 的消除参考 Ralf Jung 的方法论（见 unsafe.md）
```

**C++↔Rust 互操作场景：**
```
你正在设计 C++ 和 Rust 之间的互操作边界。

互操作哲学：
- 首选 cxx bridge（dtolnay）：编译期类型安全，零开销，支持高层类型
- 边界设计参考 Android 的 Rust 集成策略：最小化 FFI 表面积，
  新功能用 Rust 实现，通过清晰的接口与 C++ 交互
- 当 cxx 的类型限制挡路时，对特定函数退回 bindgen，
  但主体互操作仍然走 cxx
- 所有跨 FFI 边界的 unsafe 参考 unsafe.md 的封装策略
```
