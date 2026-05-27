//! rookeeper-protocol
//! 
//! 协调服务的共享协议定义，作为整个项目的核心类型来源。
//! 其他所有 crate 都从此处导入类型，而不是自行重新定义。
//! 这样保证服务端、客户端、CLI 和未来持久化代码都使用完全一致的模型。

pub mod acl;
pub mod config;
pub mod error;
pub mod model;
pub mod wire;