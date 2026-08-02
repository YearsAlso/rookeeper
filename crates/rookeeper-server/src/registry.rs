//! registry.rs
//!
//! 服务注册与发现模块。
//!
//! 设计原则：
//! - 服务实例存储在 TreeKv 的 `/services/{name}/{instance-id}` 路径下（ephemeral 节点）
//! - 实例元数据（地址、端口、健康状态）存储在节点值中
//! - 服务发现通过 List 操作实现，返回所有在线实例
//! - 实例下线时节点自动删除（ephemeral 特性）

use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use rookeeper_protocol::error::ErrorCode;
use rookeeper_protocol::model::{NodeKind, NodePath};

/// 服务实例元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    /// 实例 ID（唯一标识，由客户端生成）
    pub instance_id: String,
    /// 服务名
    pub service_name: String,
    /// 实例地址
    pub host: String,
    /// 端口
    pub port: u16,
    /// 健康状态
    pub health: HealthStatus,
    /// 额外元数据（JSON）
    pub metadata: HashMap<String, String>,
}

/// 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// 服务发现结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstances {
    pub service_name: String,
    pub instances: Vec<ServiceInstance>,
}

/// 服务注册管理器
///
/// 管理服务注册、注销和查询。
/// 服务实例以 ephemeral 节点形式存储在 TreeKv 中，
/// 实例下线时自动清理。
#[derive(Debug)]
pub struct ServiceRegistry {
    /// 服务名 → 实例 ID → ServiceInstance
    /// 这是内存索引，加速查询；实际数据在 TreeKv 中
    instances: RwLock<HashMap<String, HashMap<String, ServiceInstance>>>,
}

impl ServiceRegistry {
    /// 创建服务注册管理器
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
        }
    }

    /// 注册服务实例
    ///
    /// 将实例元数据写入内存索引（实际节点由调用方写入 TreeKv）。
    /// 如果实例 ID 已存在，则更新元数据。
    pub async fn register(
        &self,
        instance: ServiceInstance,
    ) -> Result<NodePath, ErrorCode> {
        let service_name = instance.service_name.clone();
        let instance_id = instance.instance_id.clone();

        let path = NodePath::parse(&format!(
            "/services/{}/{}",
            service_name, instance_id
        ))
        .map_err(|_| ErrorCode::InvalidPath)?;

        let mut instances = self.instances.write().await;
        instances
            .entry(service_name.clone())
            .or_default()
            .insert(instance_id, instance);

        Ok(path)
    }

    /// 注销服务实例
    pub async fn unregister(
        &self,
        service_name: &str,
        instance_id: &str,
    ) -> Result<(), ErrorCode> {
        let mut instances = self.instances.write().await;

        let service_instances = instances
            .get_mut(service_name)
            .ok_or(ErrorCode::ServiceNotFound)?;

        if service_instances.remove(instance_id).is_none() {
            return Err(ErrorCode::ServiceNotFound);
        }

        // 如果服务没有实例了，删除服务条目
        if service_instances.is_empty() {
            instances.remove(service_name);
        }

        Ok(())
    }

    /// 列举指定服务的所有在线实例
    pub async fn list(&self, service_name: &str) -> Result<ServiceInstances, ErrorCode> {
        let instances = self.instances.read().await;

        let service_instances: Vec<ServiceInstance> = instances
            .get(service_name)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();

        Ok(ServiceInstances {
            service_name: service_name.to_string(),
            instances: service_instances,
        })
    }

    /// 列举所有已注册的服务名
    pub async fn list_services(&self) -> Vec<String> {
        let instances = self.instances.read().await;
        instances.keys().cloned().collect()
    }

    /// 获取指定服务的实例数
    pub async fn instance_count(&self, service_name: &str) -> usize {
        let instances = self.instances.read().await;
        instances
            .get(service_name)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// 获取指定实例
    pub async fn get(
        &self,
        service_name: &str,
        instance_id: &str,
    ) -> Option<ServiceInstance> {
        let instances = self.instances.read().await;
        instances
            .get(service_name)
            .and_then(|m| m.get(instance_id))
            .cloned()
    }

    /// 更新实例健康状态
    pub async fn update_health(
        &self,
        service_name: &str,
        instance_id: &str,
        health: HealthStatus,
    ) -> Result<(), ErrorCode> {
        let mut instances = self.instances.write().await;

        let instance = instances
            .get_mut(service_name)
            .and_then(|m| m.get_mut(instance_id))
            .ok_or(ErrorCode::ServiceNotFound)?;

        instance.health = health;
        Ok(())
    }

    /// 清理所有已注册实例（用于测试或重置）
    pub async fn clear(&self) {
        let mut instances = self.instances.write().await;
        instances.clear();
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_instance(service: &str, id: &str) -> ServiceInstance {
        ServiceInstance {
            instance_id: id.to_string(),
            service_name: service.to_string(),
            host: "127.0.0.1".to_string(),
            port: 8080,
            health: HealthStatus::Healthy,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_register_and_list() {
        let registry = ServiceRegistry::new();
        registry
            .register(make_instance("api-gateway", "i1"))
            .await
            .unwrap();
        registry
            .register(make_instance("api-gateway", "i2"))
            .await
            .unwrap();

        let result = registry.list("api-gateway").await.unwrap();
        assert_eq!(result.instances.len(), 2);
    }

    #[tokio::test]
    async fn test_register_returns_valid_path() {
        let registry = ServiceRegistry::new();
        let path = registry
            .register(make_instance("auth", "a1"))
            .await
            .unwrap();
        assert_eq!(path.as_str(), "/services/auth/a1");
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = ServiceRegistry::new();
        registry
            .register(make_instance("cache", "c1"))
            .await
            .unwrap();
        registry
            .register(make_instance("cache", "c2"))
            .await
            .unwrap();

        registry.unregister("cache", "c1").await.unwrap();

        let result = registry.list("cache").await.unwrap();
        assert_eq!(result.instances.len(), 1);
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_returns_error() {
        let registry = ServiceRegistry::new();
        let result = registry.unregister("unknown", "x").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_services() {
        let registry = ServiceRegistry::new();
        registry
            .register(make_instance("svc-a", "x"))
            .await
            .unwrap();
        registry
            .register(make_instance("svc-b", "y"))
            .await
            .unwrap();

        let names = registry.list_services().await;
        assert!(names.contains(&"svc-a".to_string()));
        assert!(names.contains(&"svc-b".to_string()));
    }

    #[tokio::test]
    async fn test_update_health() {
        let registry = ServiceRegistry::new();
        registry
            .register(make_instance("db", "d1"))
            .await
            .unwrap();

        registry
            .update_health("db", "d1", HealthStatus::Unhealthy)
            .await
            .unwrap();

        let inst = registry.get("db", "d1").await.unwrap();
        assert_eq!(inst.health, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_update_health_nonexistent() {
        let registry = ServiceRegistry::new();
        let result = registry
            .update_health("foo", "bar", HealthStatus::Healthy)
            .await;
        assert!(result.is_err());
    }
}
