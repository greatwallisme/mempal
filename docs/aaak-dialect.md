# AAAK 方言设计文档

> mempal 中 AAAK 的完整设计说明。写给需要理解"这东西是什么、怎么工作、为什么这样设计"的人。

---

## 一句话总结

AAAK 是一种**面向 AI 的速记格式**。它把自然语言文本压缩成结构化的管道分隔行，让任何 LLM 都能快速理解上下文摘要。它**不是存储格式**——原文永远保存在 drawer 里，AAAK 只在输出侧使用（`wake-up --format aaak` 和 `compress` 命令）。

---

## 来源与改进

AAAK 最初由 MemPalace（Python）设计。那个版本有三个核心缺陷：

| 缺陷 | MemPalace (Python) | mempal (Rust) |
|------|-------------------|---------------|
| 没有解码器 | 只有 `compress()`，无法验证信息是否丢失 | 有 `decode()` + `verify_roundtrip()` |
| 没有形式语法 | 格式只存在于代码逻辑里 | 有 BNF 可解析的格式 + `parse()` |
| 没有版本号 | 无法区分不同版本的输出 | 头部行以 `V1\|` 开头 |

mempal 的 AAAK 实现修复了这三个缺陷，同时保留了原始设计的核心洞见：**"极度缩写的英语，让 LLM 当解码器"**。

---

## 格式规范

一个完整的 AAAK 文档长这样：

```
V1|myapp|auth|2026-04-08|readme
0:KAI+CLK|clerk_auth|"Kai recommended Clerk over Auth0 based on pricing and DX"|★★★★|determ|DECISION
1:KAI|auth_rollout|"roll out Clerk next sprint"|★★★|relief|TECHNICAL
T:0<->1|auth_link
ARC:anx->determ->relief
```

### 头部行

```
V{version}|{wing}|{room}|{date}|{source}
```

| 字段 | 说明 | 示例 |
|------|------|------|
| version | 格式版本，当前为 1 | `V1` |
| wing | 所属翼（项目或领域） | `myapp` |
| room | 所属房间（子领域） | `auth` |
| date | 日期或时间戳 | `2026-04-08` |
| source | 来源标识 | `readme` |

### Zettel 行（记忆卡片）

```
{id}:{entities}|{topics}|"{quote}"|{weight}|{emotions}|{flags}
```

这是 AAAK 的核心——每行是一张"记忆卡片"（Zettel）：

| 字段 | 格式 | 说明 |
|------|------|------|
| id | 数字 | 从 0 开始的递增 ID |
| entities | `AAA+BBB+CCC` | 3 字母大写实体编码，`+` 分隔 |
| topics | `topic1_topic2` | 话题关键词，`_` 分隔 |
| quote | `"原文内容"` | 双引号包裹的原文（内部双引号替换为单引号） |
| weight | `★` 到 `★★★★★` | 重要性 1-5 级 |
| emotions | `emo1+emo2` | 情感编码，`+` 分隔 |
| flags | `FLAG1+FLAG2` | 语义标志，`+` 分隔 |

### Tunnel 行（关联线）

```
T:{left}<->{right}|{label}
```

连接两个 Zettel，表示它们之间存在语义关联。`left` 和 `right` 是 Zettel ID。

### Arc 行（情感弧线）

```
ARC:{emotion1}->{emotion2}->{emotion3}
```

描述一段经历中的情感变化轨迹。箭头表示时间流向。

---

## 六种核心语法元素

### 1. 三字母实体编码

人名和专有名词被压缩为 3 个大写字母：

```
Kai → KAI    Clerk → CLK    Auth0 → AUT
```

规则：取前 3 个 ASCII 字母并大写。如果有预定义的 entity_map，优先使用映射。

对于中文实体（如"张三"），由于没有 ASCII 字母，使用稳定哈希生成 3 字母编码。如果没有检测到任何实体，使用 `UNK` 占位。

**为什么是 3 个字母？** 2 个太少（26^2 = 676 种，容易碰撞），4 个收益递减。3 个字母（17,576 种）在区分度和紧凑性之间取得最佳平衡，而且保持了对原名的直觉关联——看到 `KAI` 就知道是 Kai。

### 2. 管道分隔符 `|`

替代自然语言中的逗号、句号、换行。选择 `|` 而非其他符号的原因：
- 在自然语言中极少出现，不会产生歧义
- LLM 在训练数据中大量见过管道分隔格式（命令行、Markdown 表格），已经理解"竖线 = 字段边界"

### 3. 星级重要性 `★`

```
★      = 1（普通）
★★     = 2（一般）
★★★    = 3（重要，技术决策）
★★★★   = 4（关键，重大决策或转折）
★★★★★  = 5（极其重要）
```

自动推断规则：
- 含 DECISION 或 PIVOT 标志 → 4 星
- 含 TECHNICAL 标志 → 3 星
- 其他 → 2 星

### 4. 情感编码

28 种情感状态被压缩为 3-7 字符的短编码：

| 情感 | 编码 | 中文触发词 | 英文触发词 |
|------|------|-----------|-----------|
| 决心 | `determ` | 决定、确定 | decided, determined |
| 焦虑 | `anx` | 担心、焦虑 | worried, anxious |
| 兴奋 | `excite` | 兴奋 | excited |
| 喜悦 | `joy` | 开心、高兴 | happy, joy |
| 悲伤 | `grief` | 悲伤、失望 | sad, grief, disappoint |
| 惊讶 | `surpr` | 惊讶 | surprised |
| 感恩 | `grat` | 感恩、感谢 | grateful |
| 好奇 | `curious` | 好奇 | curious |
| 信任 | `trust` | 信任 | trust |
| 释然 | `relief` | 轻松、放心 | relieved |
| 恐惧 | `fear` | 恐惧、害怕 | fear |
| 绝望 | `despair` | 绝望 | despair |
| 热情 | `passion` | 热情 | passion |
| ... | ... | ... | ... |

如果文本中没有检测到任何情感信号，默认使用 `determ`。

### 5. 语义标志（Flags）

6 种固定标志，标记信息的类型：

| 标志 | 含义 | 触发词（英/中） |
|------|------|----------------|
| `DECISION` | 显式决策 | decided, chose, switch / 决定, 选择, 切换, 推荐 |
| `ORIGIN` | 起源时刻 | created, founded, launched / 创建, 创立, 第一次 |
| `CORE` | 核心信念 | fundamental, essential, principle / 核心, 基本, 原则 |
| `PIVOT` | 转折点 | turning point, breakthrough / 转折, 突破, 顿悟 |
| `TECHNICAL` | 技术细节 | api, database, architecture / 接口, 数据库, 架构, 部署 |
| `SENSITIVE` | 敏感内容 | password, secret, credential / 密码, 密钥, 凭证, 隐私 |

如果没有检测到任何标志，默认使用 `CORE`。

### 6. 箭头因果关系 `->`

在 Arc 行中表示情感的时间流向：

```
ARC:anx->determ->relief    （从焦虑 → 决心 → 释然）
```

---

## 编码流水线

`AaakCodec::encode(text, meta)` 的执行过程：

```
原始文本
  │
  ├─ 1. normalize_whitespace()     合并空白、替换双引号
  │
  ├─ 2. extract_entities()         英文检测大写词；中文用 jieba POS 标注（nr/ns/nt/nz）→ 3 字母编码
  │
  ├─ 3. extract_topics()           英文分词 + jieba 中文分词 → POS 过滤（保留 n*/v*/a*）→ 取前 3 个
  │
  ├─ 4. detect_emotions()          关键词匹配 → 情感编码（中英双语）
  │
  ├─ 5. detect_flags()             关键词匹配 → 语义标志（中英双语）
  │
  ├─ 6. infer_weight()             根据标志推断重要性
  │
  └─ 7. 组装 AaakDocument          头部行 + Zettel 行 + 往返验证报告
```

输出是 `EncodeOutput`，包含：
- `document: AaakDocument` — 编码后的文档
- `report: EncodeReport` — 编码报告（话题截断数、覆盖率、丢失的断言）

## 解码流水线

`AaakCodec::decode(document)` 的执行过程：

```
AaakDocument
  │
  ├─ 遍历每个 Zettel
  │   │
  │   ├─ 取 quote 字段
  │   │
  │   └─ 如果有 entity_map，将 3 字母编码还原为原名
  │       例如：KAI → Kai, CLK → Clerk
  │
  └─ 所有 zettel 的解码文本用换行连接
```

## 往返验证

`verify_roundtrip(original, document)` 检查编码后信息是否丢失：

1. 将原文按句号/感叹号/问号/分号（含中文全角标点）拆分为**断言列表**
2. 解码 document 得到还原文本
3. 逐一检查每个断言是否出现在还原文本中
4. 返回 `RoundtripReport`：
   - `preserved`: 保留的断言
   - `lost`: 丢失的断言
   - `coverage`: 保留率（0.0 - 1.0）

spec 要求 coverage >= 0.8。

---

## 解析（从字符串到结构体）

`AaakDocument::parse(input)` 可以从 AAAK 字符串重建结构体：

```rust
let doc = AaakDocument::parse(
    "V1|myapp|auth|2026-04-08|readme\n\
     0:KAI|clerk_auth|\"use Clerk\"|★★★★|determ|DECISION"
)?;
assert_eq!(doc.header.version, 1);
assert_eq!(doc.zettels[0].entities, vec!["KAI"]);
```

解析器会验证：
- 实体编码必须是 3 个大写字母
- 情感编码必须是 3-7 个小写字母
- 标志必须是已知的 6 种之一
- 星级必须是 1-5 个 ★
- Tunnel 引用的 Zettel ID 必须存在
- 不允许重复的 Zettel ID

---

## 中文支持

AAAK 使用 **jieba-rs** 做真正的中文分词和词性标注，而不是 bigram 启发式：

| 能力 | 英文策略 | 中文策略 |
|------|---------|---------|
| 实体检测 | 首字母大写词 | jieba POS 标注（`nr*` 人名 / `ns` 地名 / `nt` 组织 / `nz` 专名） |
| 实体编码 | 取前 3 字母大写 | 稳定哈希生成 3 字母 ASCII 码 |
| 话题提取 | 按空格/标点分词 | jieba 分词 + POS 过滤（保留 `n*` / `v*` / `a*`） |
| 停用词 | 11 个英文虚词 | jieba POS 自动过滤 + 人工兜底列表 |
| 情感检测 | 37 个英文关键词 | 30 个中文关键词（作为 jieba 之上的补充） |
| 标志检测 | 24 个英文关键词 | 31 个中文关键词 |
| 断言拆分 | `. ! ? ;` | `. ! ? ; 。！？；，` |

**示例**：输入"阿里巴巴集团在杭州发布了新的云服务产品"，jieba 会识别出：
- 实体：`阿里巴巴`（`nz` 专有名词）、`杭州`（`ns` 地名）
- 话题：`集团`、`发布`、`服务`（均为 `n` 或 `v`）
- 自动跳过 `的`、`了`、`在`、`新的` 等虚词

**实现细节**：
- jieba 词典懒加载（`OnceLock`），首次调用时初始化，后续复用
- 实体和话题互斥——专有名词只进 entities，普通名词/动词只进 topics
- 对 jieba 默认词典里没有的人名（如 "李四"、"赵雷"），建议通过 `entity_map` 预定义别名

**成本**：引入 jieba-rs 增加约 5MB 的词典到二进制，换来准确的中文分词能力。如果目标是极小体积部署，可以 fork 并加 feature flag 禁用 jieba。

---

## AAAK 在 mempal 中的位置

```
                    ┌─────────────┐
                    │  CLI / MCP  │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
         wake-up      compress       search
         --format       命令          引擎
          aaak                    （不用 AAAK）
              │            │
              └────────────┘
                    │
            ┌───────┴───────┐
            │  mempal-aaak  │  ← 只在输出侧
            └───────────────┘

       ┌─────────────────────────────┐
       │       mempal-core           │
       │   drawers 表存原文          │  ← 永远保留完整内容
       │   drawer_vectors 存向量     │
       └─────────────────────────────┘
```

关键架构约束：
- **AAAK 不被 ingest 或 search 依赖**
- **数据永远 raw 存储**——drawer 保存原文，AAAK 只是输出时的可选格式化
- **AAAK 是输出格式化器，不是存储格式**

---

## 设计哲学

AAAK 的核心洞见：**LLM 就是解码器**。

传统压缩（gzip、zstd）需要专门的解码器。AAAK 不需要——任何能读英文的 LLM 看到 `KAI(backend,3yr)` 就知道"Kai 做后端，3 年经验"。这不是巧合，而是刻意的设计：

- 三字母编码利用了"大写缩写 = 名字"的直觉
- 管道分隔利用了"竖线 = 字段边界"的直觉
- 星级利用了"更多星 = 更重要"的直觉
- 情感编码利用了"缩写 = 原词"的直觉

每一种语法元素都建立在 LLM 训练数据中已有的模式上，不需要学习新规则。

**最诚实的定位**：AAAK 是面向 AI 的速记索引格式。它的核心价值不在于"无损压缩"，而在于"跨模型可读的高效上下文摘要"。原文始终在 drawer 里，AAAK 帮你在有限的 context window 里塞进更多有用信息。
