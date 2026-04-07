# 异步 Rust

## 概念锚点

### 运行时与模式
参考 **Alice Ryhl**（Tokio 核心维护者）的最佳实践：

- `spawn` 用于真正需要并发的任务，不是每个 async fn 都要 spawn
- `spawn_blocking` 用于 CPU 密集或同步 I/O 操作——阻塞异步运行时是性能杀手
- 取消安全性（cancellation safety）是必须考虑的——`tokio::select!` 的每个分支在被取消时不能留下不一致状态
- Channel 选型：`mpsc` 用于多生产者单消费者，`broadcast` 用于多消费者需要每条消息，`watch` 用于只关心最新值

### 框架选型
参考 **axum**（Tokio 团队出品）的组合式设计：

- 中间件是函数组合，不是继承层级
- 从 `tower::Service` trait 生长出来的设计——每层中间件只做一件事
- handler 用 `impl IntoResponse` 返回值而非手写 Response 构造
- 提取器（Extractor）模式：用类型系统表达"这个 handler 需要什么输入"

参考 **Sean McArthur** 的 `hyper` 做底层 HTTP 需求——`axum` 和 `hyper` 的关系是高层/底层抽象的典范。

### Stream 处理
参考 `tokio-stream` 和 `futures::Stream`：

- Stream 是异步的 Iterator——同样的组合子思路（map, filter, take）
- 背压（backpressure）是必须设计的，不是事后补的

## 常见决策点

**什么时候用 async，什么时候不用？**

async 适合 I/O 密集型工作负载。如果你的程序主要在等待网络、文件系统、数据库，async 能让一个线程服务大量并发连接。如果主要是 CPU 计算，普通线程 + `rayon` 更简单也更快。参考 Rich Hickey 的判断：async 是 simple 还是仅仅 easy？在很多场景下，同步代码 + 线程池就是那个更 simple 的方案。

**Tokio vs async-std vs smol？**

生态决定选择。Tokio 的生态最大（`axum`, `tonic`, `sqlx`, `reqwest` 都基于 Tokio），选它几乎不会错。只在极端的二进制体积要求下考虑 `smol`。

**怎么做 graceful shutdown？**

参考 Tokio 的 mini-redis 示例项目——用 `tokio::signal` 监听关闭信号，通过 `broadcast` channel 或 `CancellationToken` 通知所有任务优雅退出。这比 `process::exit()` 健壮得多。
