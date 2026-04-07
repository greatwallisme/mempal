# Web 服务

## 概念锚点

### 框架
参考 **axum**（Tokio 团队出品）的设计哲学：

- 从 `tower::Service` trait 生长出来——中间件是函数组合，不是框架魔法
- Extractor 模式：handler 的参数类型声明了它需要的输入，框架负责提取和验证
- `impl IntoResponse` 让任何类型都可以成为响应——包括自定义错误类型
- 路由是数据，不是宏——用 `Router::new().route()` 组合

如果 axum 的抽象层级不够低，退到 **hyper**（Sean McArthur）做底层控制。

### 数据库
参考 **sqlx** 的编译期查询检查：

- 用 `sqlx::query!` 宏在编译期验证 SQL 语句的正确性和类型匹配
- 迁移用 `sqlx migrate` 管理
- 连接池是默认选项，不是优化选项

### API 设计
参考 **REST API 的 BurntSushi 式错误处理**：

- HTTP 错误响应和内部错误类型分离——不要把 `thiserror` 的 Display 直接暴露给客户端
- 错误响应有结构化的 JSON body（error code + message + optional details）
- 用中间件统一处理错误转换，handler 只管返回 `Result`

## 典型依赖组合

```toml
# Web 框架
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "compression-gzip"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 数据库
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "migrate"] }

# 异步运行时
tokio = { version = "1", features = ["full"] }

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# 配置
dotenvy = "0.15"
```
