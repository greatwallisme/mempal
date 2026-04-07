# 嵌入式 Rust

## 概念锚点

### 基础架构
参考 **Jorge Aparicio (japaric)** 奠定的嵌入式 Rust 范式：

- 用 trait 抽象硬件（`embedded-hal`）——驱动代码面向 trait 而非具体芯片
- 用类型系统编码引脚状态——编译期防止把输入引脚当输出用
- PAC (Peripheral Access Crate) + HAL 分层——PAC 由 `svd2rust` 从厂商数据自动生成，HAL 在其上提供人类友好的 API

### 实用主义
参考 **James Munns**（OneVariable / `postcard`）的方法论：

- `no_std` 不是教条——如果目标平台有足够内存，`alloc` 甚至有限的 `std` 也是合理选择
- 嵌入式代码的第一优先级是在目标硬件上可靠运行，不是追求零成本抽象的极致
- 序列化用 `postcard`（极小二进制体积、no_std 友好），不要在 MCU 上用 JSON

### 并发模型
参考 **RTIC** vs **Embassy** 的取舍：

- **RTIC**：编译期资源分析，中断驱动，零运行时开销。适合硬实时、资源极度受限的场景。用 Rust 类型系统替代 RTOS
- **Embassy**：嵌入式 async/await 运行时，协程模型。适合复杂异步流程（蓝牙+WiFi+传感器同时工作）。创始人 Dario Nieuwenhuis 做了大量"嵌入式 async 该长什么样"的设计取舍

选型判断：任务间依赖关系简单、硬实时要求高 → RTIC；多种异步 I/O 混合、需要状态机管理 → Embassy。

### 日志和调试
参考 **defmt**（Ferrous Systems / Knurling 项目）的零成本哲学：

- 格式化在主机端完成，MCU 端只传输极小的编码数据
- 比 `log` + `semihosting` 快一到两个数量级
- 配合 `probe-rs` 做烧录和实时日志

## 工具链

```bash
# 交叉编译目标
rustup target add thumbv7em-none-eabihf    # Cortex-M4F/M7
rustup target add thumbv6m-none-eabi        # Cortex-M0/M0+
rustup target add riscv32imc-unknown-none-elf  # RISC-V

# 构建
cargo build --target thumbv7em-none-eabihf --release

# 烧录和调试（probe-rs）
cargo embed --release          # 烧录 + RTT 日志
cargo flash --release --chip STM32F411CEUx  # 仅烧录

# 二进制体积分析
cargo size --release -- -A     # 段大小
cargo bloat --release          # 函数级体积分析
```

## Cargo.toml 嵌入式模板

```toml
[profile.release]
opt-level = "z"       # 优化体积
lto = true            # 链接时优化
codegen-units = 1     # 单代码生成单元，更好的优化
debug = true          # release 也保留调试信息（不影响二进制大小）
panic = "abort"       # 不用 unwind，节省空间

[profile.dev]
opt-level = 1         # dev 也做基本优化，否则某些嵌入式代码太慢
```
