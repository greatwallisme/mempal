# CLI 工具

## 概念锚点

### 整体哲学
参考 **BurntSushi** 的 CLI 工具设计（`ripgrep`, `xsv`）：

- 零配置即可用——合理的默认值让用户不读文档也能完成 80% 的工作
- 错误消息是用户界面——错误输出必须告诉用户发生了什么、在哪里发生的、能做什么
- 性能是功能——用户能感知的延迟就是 bug
- 遵循 Unix 哲学：stdin/stdout 管道、非零 exit code 表示错误、stderr 留给人读的消息

### 参数解析
参考 **clap**（Ed Page 维护）的 derive API：

- 用 `#[derive(Parser)]` 声明式定义参数——类型系统自动处理解析和验证
- 子命令用 enum + derive 表达，每个变体就是一个子命令
- `--help` 和 shell 补全自动生成

### TUI（终端界面）
参考 **ratatui** 的即时模式渲染：

- 每帧完全重绘，不维护 UI 状态树
- 布局用约束系统（类似 CSS flexbox）
- 事件处理和渲染分离

### 错误输出
参考 BurntSushi 在 ripgrep 中的实践——CLI 工具的错误输出应该遍历整条错误链：

```rust
fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        let mut source = e.source();
        while let Some(cause) = source {
            eprintln!("  caused by: {cause}");
            source = cause.source();
        }
        std::process::exit(1);
    }
}
```

## 典型依赖组合

```toml
# 参数解析
clap = { version = "4", features = ["derive"] }

# 错误处理（应用级）
anyhow = "1"

# 终端输出
colored = "2"           # 简单着色
indicatif = "0.17"      # 进度条

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"
csv = "1"               # BurntSushi 的 CSV 库

# TUI（如需要）
ratatui = "0.29"
crossterm = "0.28"
```
