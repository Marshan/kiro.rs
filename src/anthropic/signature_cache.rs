//! 签名缓存管理器
//!
//! 封装了 thinking 签名的全局缓存，按 conversation_id 分组管理。
//! - 以 conversation_id 为粒度进行过期清理（TTL）
//! - 总条目数超过上限时，按 conversation_id 最久未更新的顺序整组淘汰

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

use parking_lot::RwLock;

/// 单个会话的签名缓存
struct ConversationCache {
    /// thinking_content -> signature
    signatures: HashMap<String, String>,
    /// 该会话最后一次写入/访问的时间戳
    last_accessed: Instant,
}

/// 缓存配置
struct CacheConfig {
    /// 最大缓存条目数（所有会话的签名总数）
    max_entries: usize,
    /// 过期时间（秒）
    ttl_secs: u64,
}

/// 默认最大缓存条目数
const DEFAULT_MAX_ENTRIES: usize = 10000;

/// 默认过期时间：12 小时
const DEFAULT_TTL_SECS: u64 = 12 * 60 * 60;

/// 清理间隔：1 小时
pub const CLEANUP_INTERVAL_SECS: u64 = 60 * 60;

/// 全局缓存实例：conversation_id -> ConversationCache
static CACHE: OnceLock<RwLock<HashMap<String, ConversationCache>>> = OnceLock::new();

/// 全局配置
static CONFIG: OnceLock<CacheConfig> = OnceLock::new();

fn cache() -> &'static RwLock<HashMap<String, ConversationCache>> {
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn config() -> &'static CacheConfig {
    CONFIG.get_or_init(|| CacheConfig {
        max_entries: DEFAULT_MAX_ENTRIES,
        ttl_secs: DEFAULT_TTL_SECS,
    })
}

/// 初始化缓存配置（应在程序启动时调用一次）
///
/// # Arguments
/// * `max_entries` - 最大缓存条目数，None 使用默认值 10000
/// * `ttl_hours` - 过期时间（小时），None 使用默认值 12
pub fn init(max_entries: Option<usize>, ttl_hours: Option<u64>) {
    let _ = CONFIG.set(CacheConfig {
        max_entries: max_entries.unwrap_or(DEFAULT_MAX_ENTRIES),
        ttl_secs: ttl_hours.unwrap_or(12) * 3600,
    });
}

/// 计算所有会话的签名总条目数
fn total_entries(map: &HashMap<String, ConversationCache>) -> usize {
    map.values().map(|c| c.signatures.len()).sum()
}

/// 签名缓存管理器 — 对外暴露的高阶接口
pub struct SignatureCacheManager;

impl SignatureCacheManager {
    /// 插入签名
    ///
    /// 当总条目数超过上限时，按 conversation_id 最久未访问的顺序整组淘汰。
    pub fn insert(conv_id: String, text: String, sig: String) {
        let cfg = config();
        let mut map = cache().write();

        let conv = map.entry(conv_id).or_insert_with(|| ConversationCache {
            signatures: HashMap::new(),
            last_accessed: Instant::now(),
        });
        conv.signatures.insert(text, sig);
        conv.last_accessed = Instant::now();

        // 超过上限时，按 conversation_id 最久未更新的顺序整组淘汰
        if total_entries(&map) > cfg.max_entries {
            // 收集所有 conversation_id 按 last_accessed 升序排列
            let mut conv_ids: Vec<_> = map
                .iter()
                .map(|(id, c)| (id.clone(), c.last_accessed))
                .collect();
            conv_ids.sort_by_key(|(_, t)| *t);

            let mut current_total = total_entries(&map);
            let mut removed_convs = 0usize;
            for (id, _) in conv_ids {
                if current_total <= cfg.max_entries {
                    break;
                }
                if let Some(removed) = map.remove(&id) {
                    current_total -= removed.signatures.len();
                    removed_convs += 1;
                }
            }
            if removed_convs > 0 {
                tracing::info!(
                    "签名缓存容量清理: 淘汰 {} 个会话, 剩余 {} 条",
                    removed_convs,
                    total_entries(&map)
                );
            }
        }
    }

    /// 触发会话更新（如客户端有新请求，或模型 API 有返回时）
    ///
    /// 若会话已存在，仅更新其 `last_accessed` 时间；
    /// 若不存在，则创建一个空的会话缓存并设置其 `last_accessed`。
    pub fn touch(conv_id: String) {
        if conv_id.is_empty() {
            return;
        }
        let mut map = cache().write();
        let conv = map.entry(conv_id).or_insert_with(|| ConversationCache {
            signatures: HashMap::new(),
            last_accessed: Instant::now(),
        });
        conv.last_accessed = Instant::now();
    }

    /// 查询签名，命中时更新该会话的 last_accessed
    pub fn get(conv_id: &str, text: &str) -> Option<String> {
        let mut map = cache().write();
        if let Some(conv) = map.get_mut(conv_id) {
            if let Some(sig) = conv.signatures.get(text) {
                conv.last_accessed = Instant::now();
                return Some(sig.clone());
            }
        }
        None
    }

    /// 清理过期条目（由后台任务定期调用）
    ///
    /// 移除所有 last_accessed 超过 TTL 的会话
    pub fn cleanup_expired() {
        let cfg = config();
        let mut map = cache().write();
        let before = map.len();
        let before_entries = total_entries(&map);
        let deadline = Instant::now() - std::time::Duration::from_secs(cfg.ttl_secs);
        map.retain(|_, conv| conv.last_accessed > deadline);
        let removed_convs = before - map.len();
        if removed_convs > 0 {
            tracing::info!(
                "签名缓存定时清理: 移除 {} 个过期会话 ({} 条签名), 剩余 {} 个会话 ({} 条)",
                removed_convs,
                before_entries - total_entries(&map),
                map.len(),
                total_entries(&map)
            );
        }
    }

    /// 返回当前缓存条目总数（供监控使用）
    #[allow(dead_code)]
    pub fn len() -> usize {
        total_entries(&cache().read())
    }

    /// 返回当前缓存的会话数（供监控使用）
    #[allow(dead_code)]
    pub fn conversation_count() -> usize {
        cache().read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        SignatureCacheManager::insert("c1".into(), "text1".into(), "sig1".into());
        assert_eq!(
            SignatureCacheManager::get("c1", "text1"),
            Some("sig1".into())
        );
        assert_eq!(SignatureCacheManager::get("c1", "missing"), None);
        assert_eq!(SignatureCacheManager::get("missing", "text1"), None);
    }

    #[test]
    fn test_same_conversation_multiple_signatures() {
        SignatureCacheManager::insert("c2".into(), "t1".into(), "s1".into());
        SignatureCacheManager::insert("c2".into(), "t2".into(), "s2".into());
        assert_eq!(SignatureCacheManager::get("c2", "t1"), Some("s1".into()));
        assert_eq!(SignatureCacheManager::get("c2", "t2"), Some("s2".into()));
    }

    #[test]
    fn test_fallback_after_miss() {
        // 未命中返回 None，调用方应使用 fallback 签名
        let result = SignatureCacheManager::get("nonexistent", "text");
        assert_eq!(result, None);
    }

    #[test]
    fn test_touch() {
        SignatureCacheManager::touch("c_touch_new".into());
        let map = cache().read();
        let conv = map.get("c_touch_new").unwrap();
        assert_eq!(conv.signatures.len(), 0);
    }
}
