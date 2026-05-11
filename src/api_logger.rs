use std::sync::Arc;
use tokio::sync::mpsc;

/// API 调用日志记录器
///
/// 通过 tokio channel 异步写入日志文件，不阻塞请求路径。
/// channel 满时静默丢弃，保证请求延迟不受日志写入影响。
#[derive(Clone)]
pub struct ApiLogger {
    sender: mpsc::Sender<String>,
}

impl ApiLogger {
    /// 创建日志记录器并启动后台写文件 task
    pub fn new(path: &str) -> Self {
        let (sender, mut receiver) = mpsc::channel::<String>(1024);
        let path = path.to_string();

        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await;

            match file {
                Ok(mut f) => {
                    while let Some(entry) = receiver.recv().await {
                        let _ = f.write_all(entry.as_bytes()).await;
                        let _ = f.flush().await;
                    }
                }
                Err(e) => {
                    tracing::error!("无法打开 API 日志文件 {}: {}", path, e);
                }
            }
        });

        Self { sender }
    }

    /// 异步投递日志条目，channel 满时静默丢弃
    pub fn log(&self, entry: String) {
        let _ = self.sender.try_send(entry);
    }

    /// 格式化客户端请求日志条目
    pub fn format_request(
        request_id: &str,
        model_in: &str,
        model_kiro: &str,
        stream: bool,
        max_tokens: i32,
        messages: &[crate::anthropic::types::Message],
    ) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let mut s = format!(
            "[{now}] ── REQUEST ──────────────────────────────────\n\
             {:<12}: {request_id}\n\
             {:<12}: {model_in}\n\
             {:<12}: {model_kiro}\n\
             {:<12}: {stream}\n\
             {:<12}: {max_tokens}\n\
             {:<12}: [{} messages]\n",
            "request_id",
            "model_in",
            "model_kiro",
            "stream",
            "max_tokens",
            "messages",
            messages.len(),
        );
        for (i, msg) in messages.iter().enumerate() {
            let role = &msg.role;
            let content = extract_message_text(&msg.content);
            let content = truncate(content, 500);
            s.push_str(&format!("  [{i}] {role:<10}: {content}\n"));
        }
        s.push('\n');
        s
    }

    /// 格式化 Kiro 请求日志条目
    pub fn format_kiro_request(request_id: &str, url: &str, body: &str) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        format!(
            "[{now}] ── KIRO REQUEST ─────────────────────────────\n\
             {:<12}: {request_id}\n\
             {:<12}: {url}\n\
             {:<12}: {body}\n\n",
            "request_id", "url", "body",
        )
    }

    /// 格式化响应日志条目
    #[allow(clippy::too_many_arguments)]
    pub fn format_response(
        request_id: &str,
        model_in: &str,
        model_kiro: &str,
        stop_reason: &str,
        input_tokens: i32,
        output_tokens: i32,
        duration_ms: u128,
        content: &str,
    ) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        format!(
            "[{now}] ── RESPONSE ─────────────────────────────────\n\
             {:<12}: {request_id}\n\
             {:<12}: {model_in}\n\
             {:<12}: {model_kiro}\n\
             {:<12}: {stop_reason}\n\
             {:<12}: {input_tokens}\n\
             {:<12}: {output_tokens}\n\
             {:<12}: {duration_ms}\n\
             {:<12}: {content}\n\n",
            "request_id",
            "model_in",
            "model_kiro",
            "stop_reason",
            "input_tok",
            "output_tok",
            "duration_ms",
            "content",
        )
    }
}

/// 将字符串截断到指定字节长度（按字符边界截断）
fn truncate(s: String, max_chars: usize) -> String {
    let s = s.replace('\n', "\\n");
    if s.chars().count() <= max_chars {
        s
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}...")
    }
}

/// 从 serde_json::Value 中提取文本内容
fn extract_message_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            arr.iter()
                .filter_map(|block| {
                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                        block.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        other => other.to_string(),
    }
}

/// 创建 ApiLogger 的 Arc 包装（供 AppState 使用）
pub fn create_api_logger(path: &str) -> Arc<ApiLogger> {
    Arc::new(ApiLogger::new(path))
}
