# 密码学与安全

## 概念锚点

### 密码学原语
参考 **ring**（Brian Smith）的设计哲学：

- 密码学 API 必须难以误用——好的 API 设计让错误用法在编译期被拦截
- 不暴露底层原语给普通用户——提供 task-oriented API（"签名"、"加密"），而非算法-oriented API（"AES-CBC"）
- 永远不自己实现密码学算法——用经过审计的库

### TLS
参考 **rustls**（ctz / Joseph Birr-Pixton）：

- 纯 Rust TLS 实现，无 OpenSSL 依赖
- 默认只启用安全的密码套件——不提供不安全的选项
- 配合 `webpki` 做证书验证

### 生态分层

```
应用层：  rustls（TLS）、argon2（密码哈希）
         ↓ 构建于
中间层：  ring（密码原语）、RustCrypto 组织（纯 Rust 实现）
         ↓ 区别
ring = 性能优先，部分用汇编/C（来自 BoringSSL），经过广泛审计
RustCrypto = 纯 Rust，更多算法覆盖，社区维护
```

选型判断：生产环境 TLS/签名/AEAD → `ring`；需要更多算法覆盖或纯 Rust 要求 → RustCrypto。两者不要混用——它们的类型不兼容。

### 安全编码
- 密钥和敏感数据用 `zeroize` crate 在 drop 时清零内存
- 恒时比较（constant-time comparison）防止时序攻击——用 `subtle` crate 的 `ConstantTimeEq`
- 随机数生成用 `rand` 的 `OsRng`（操作系统熵源），永远不用 `thread_rng` 做密码学用途

## 典型依赖组合

```toml
# TLS
rustls = "0.23"
webpki-roots = "0.26"       # Mozilla 根证书

# 密码学原语
ring = "0.17"               # 或 RustCrypto 系列

# 密码哈希
argon2 = "0.5"              # 密码存储
sha2 = "0.10"               # SHA-256/512（RustCrypto）

# 安全工具
zeroize = { version = "1", features = ["derive"] }
subtle = "2"                # 恒时比较
rand = "0.8"                # 随机数

# HTTP 客户端（带 rustls）
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
```
