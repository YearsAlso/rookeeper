use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;
use crate::model::SessionId;

pub const PROTOCOL_VERSION_V1: u16 = 1;
pub const FRAME_HEADER_LEN: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum RequestKind {
    Get = 1,
    Create = 2,
    Set = 3,
    Delete = 4,
    List = 5,
    Watch = 6,
    AcquireLock = 7,
    ReleaseLock = 8,
    RegisterService = 9,
    GetStatus = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum EventKind {
    NodeCreated = 1,
    NodeUpdated = 2,
    NodeDeleted = 3,
    ChildrenChanged = 4,
    LockAcquired = 5,
    LockReleased = 6,
    ServiceRegistered = 7,
    ServiceUnregistered = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeader {
    pub version: u16,
    pub request_kind: u16,
    pub flags: u16,
    pub reserved: u16,
    pub request_id: u32,
    pub session_id: SessionId,
    pub payload_len: u32,
}

impl FrameHeader {
    pub fn new(
        kind: RequestKind,
        request_id: u32,
        session_id: SessionId,
        payload_len: u32,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION_V1,
            request_kind: kind as u16,
            flags: 0,
            reserved: 0,
            request_id,
            session_id,
            payload_len,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestFrame {
    pub header: FrameHeader,
    pub payload: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseFrame {
    pub request_id: u32,
    pub status: ErrorCode,
    pub payload: Bytes,
}
