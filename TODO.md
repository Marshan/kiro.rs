# TODO: 优化 Kiro-RS 历史签名缓存 (SIGNATURE_CACHE) 防止内存泄漏

## 1. 背景与问题背景 (Context & Background)

在最近的重构中，我们引入了原生 `reasoningContent` 历史适配。由于前端客户端（如 Claude Code CLI）遵循标准 Anthropic 消息协议，在追问时只会发送思考过程的纯文本（`thinking` 内容），并不保留服务端返回的加密签名（`signature`）。而上游 `kiro.dev` API 对历史推理文本的校验要求文本与签名并存。

为了进行适配，我们在 `kiro-rs` 的内存中引入了一个全局静态的线程安全缓存 `SIGNATURE_CACHE`：
```rust
static SIGNATURE_CACHE: OnceLock<RwLock<HashMap<(String, String), String>>> = OnceLock::new();
```

### 运作机制：
1. **写入**：网关收到 `kiro.dev` 响应（流式或非流式）时，捕获其中的加密签名，将 `(conversation_id, thinking_content) -> signature` 存入缓存。
2. **读取**：网关收到客户端下一轮请求时，提取历史中的思考内容，查阅缓存得到对应的签名，拼装成 `reasoningContent` 发送给上游。

---

## 2. 内存泄漏风险分析 (Memory Leak Risk)

当前的 `SIGNATURE_CACHE` 使用的是无界的 `HashMap`。
* **数据大小估算**：每一个历史缓存项（会话 ID + 思考文本 + 加密签名）大约占用 **1KB ~ 5KB** 的堆内存。
* **风险点**：缓存只增不减。对于个人临时运行网关，由于进程生命周期短，该问题不明显。但如果 `kiro-rs` 作为后台服务**持续不间断运行**，或者在多人、多 Agent 高频调用的生产环境中，该 `HashMap` 的尺寸会无限增长，最终导致**内存泄漏（Memory Leak）**并可能触发 OOM 崩溃。

---

## 3. 待办清单与优化方案 (TODO List & Solutions)

为了彻底解决长时运行的内存隐患，未来建议实施以下优化：

- [ ] **1. 选择并实现缓存淘汰策略**
  建议采用以下两种方案之一对 `SIGNATURE_CACHE` 进行重构：
  * **方案 A：LRU 缓存（推荐）**
    * 引入轻量级无标准依赖的 LRU 库（如 `lru` 库）或手工实现简单双向链表。
    * 限制最大条数（例如上限 2000 条历史纪录）。当达到上限时，自动淘汰最久未被访问的签名。
  * **方案 B：带 TTL 自动过期的缓存**
    * 引入 `mini-moka` 或其他支持 TTL/TTI 的缓存库。
    * 给每个签名键值对设置过期时间（如 12 小时）。由于多轮会话在用户停止提问几小时后基本失效，过期后自动释放内存。
  * **方案 C：纯原生定时后台清理（零依赖）**
    * 将缓存结构改为 `HashMap<(String, String), (String, Instant)>`（记录存入/访问时间戳）。
    * 在 `kiro-rs` 启动时，拉起一个低优先级的后台循环异步任务（或利用定时器），每隔 1 小时扫描并清除超过 12 小时未被访问的旧条目。

- [ ] **2. 封装签名缓存接口**
  * 在 [src/anthropic/converter.rs](file:///D:/code/person/kiro.rs/src/anthropic/converter.rs) 中，将当前直接操作 `HashMap` 的裸逻辑封装为高阶服务类（如 `SignatureCacheManager`）。
  * 暴露出清晰的 `.insert(conv_id, text, sig)` 和 `.get(conv_id, text)` 接口，将底层的锁（`RwLock`）与淘汰策略对业务层屏蔽。

- [ ] **3. 极限与边界测试验证**
  * 编写单元测试模拟大批量并发写入，验证缓存大小到达设定的阈值后，旧的签名能被正确逐出，且内存占用保持平稳。
  * 测试当旧签名被逐出后，多轮对话请求再次发起时，系统能平稳降级使用 `fallback` 签名，不会导致请求崩溃。
