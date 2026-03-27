use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Authentication
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CreateSessionRequest {
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionResponse {
    pub session_token: String,
    pub config: Option<SessionConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    pub agent_id: i64,
    pub project_id: i64,
}

// ---------------------------------------------------------------------------
// Heartbeat
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Online,
    Busy,
    Error,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuDevice {
    pub name: String,
    pub memory: f64,
    pub compute_capability: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub hashcat_version: Option<String>,
    pub gpu_devices: Vec<GpuDevice>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub temperature: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatRequest {
    pub status: AgentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_info: Option<DeviceInfo>,
}

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRange {
    pub start: i64,
    pub end: i64,
    pub agent_speed_hs: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResources {
    pub hash_list_url: Option<String>,
    pub wordlist_url: Option<String>,
    pub rulelist_url: Option<String>,
    pub masklist_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDescriptor {
    pub id: i64,
    pub attack_id: i64,
    pub campaign_id: i64,
    pub mode: i32,
    pub hash_type_id: i32,
    pub work_range: Option<WorkRange>,
    pub resources: Option<TaskResources>,
}

#[derive(Debug, Deserialize)]
pub struct NextTaskResponse {
    pub task: Option<TaskDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
    Exhausted,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub keyspace_progress: f64,
    pub speed: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrackResult {
    pub hash_value: String,
    pub plaintext: String,
}

#[derive(Debug, Serialize)]
pub struct TaskReport {
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<TaskProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<CrackResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Zaps (cracked hash values for skipping)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZapResponse {
    pub zaps: Vec<String>,
    pub has_more: bool,
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkEntry {
    pub hashcat_mode: i32,
    pub hash_type: String,
    pub speed_hs: i64,
    pub device_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkSubmission {
    pub entries: Vec<BenchmarkEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cracker_version: Option<String>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentErrorReport {
    pub severity: ErrorSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<i64>,
}

// ---------------------------------------------------------------------------
// Common response envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AcknowledgedResponse {
    pub acknowledged: bool,
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorDetail {
    pub code: Option<String>,
    pub message: Option<String>,
}
