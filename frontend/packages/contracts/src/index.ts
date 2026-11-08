/**
 * Language-neutral API contract types for the Axon web and mobile clients.
 * Keep these aligned with the versioned schemas in /contracts.
 */
export type Workspace = {
  id: string;
  name: string;
  created_at: string;
};

export type Agent = {
  id: string;
  workspace_id: string;
  display_name: string;
  provider: string;
  connection_mode: "manual";
  created_at: string;
};

export type Artifact = {
  id: string;
  workspace_id: string;
  title: string;
  content: string;
  source_agent_id: string | null;
  captured_at: string;
};

export type TaskStatus =
  | "offered"
  | "acknowledged"
  | "in_progress"
  | "submitted"
  | "review_required"
  | "accepted"
  | "changes_requested"
  | "completed"
  | "rejected"
  | "cancelled";

export type TaskTransition = {
  from: TaskStatus | null;
  to: TaskStatus;
  note: string;
  occurred_at: string;
};

export type Task = {
  id: string;
  workspace_id: string;
  title: string;
  objective: string;
  assignee_agent_id: string;
  context_artifact_ids: string[];
  response_artifact_id: string | null;
  status: TaskStatus;
  version: number;
  created_at: string;
  updated_at: string;
  transitions: TaskTransition[];
};

export type Health = {
  status: string;
  service: string;
  storage: "development_memory_only" | "persistent" | string;
  authentication: string;
};


export type AxonDeviceKind =
  | "mobile_robot" | "robot_arm" | "drone" | "sensor" | "simulator" | "other";

export type DeviceRegistration = {
  device_id: string;
  workspace_id: string;
  device_type: AxonDeviceKind;
  manufacturer: string;
  model: string;
  identity_key_fingerprint: string;
  capabilities: string[];
  firmware_version?: string;
  registered_at: string;
};

export type TelemetryKind =
  | "pose" | "velocity" | "joint_state" | "battery" | "temperature"
  | "safety_state" | "diagnostic" | "heartbeat";

export type TelemetryEvent = {
  event_id: string;
  device_id: string;
  sequence: number;
  observed_at: string;
  received_at: string;
  kind: TelemetryKind;
  frame_id?: string;
  payload: Record<string, unknown>;
  quality?: "measured" | "estimated" | "simulated" | "unknown";
};

export type SimulatorTelemetryIngest = {
  event_id: string;
  sequence: number;
  observed_at: string;
  kind: TelemetryKind;
  frame_id?: string;
  payload: Record<string, unknown>;
  quality: "simulated";
};

export type CommandProposal = {
  proposal_id: string;
  device_id: string;
  requester_id: string;
  operation: "move_base" | "set_joint_target" | "stop" | "home" | "custom";
  parameters: Record<string, unknown>;
  sequence: number;
  issued_at: string;
  expires_at: string;
  idempotency_key: string;
  reason: string;
  simulation_only?: true;
};

export type DeviceSafetyState = {
  device_id: string;
  emergency_stop: boolean;
  interlocks_healthy: boolean;
  operating_mode: "disabled" | "manual" | "assisted" | "autonomous" | "maintenance" | "fault";
  observed_at: string;
  source: "device_controller" | "safety_plc" | "simulator";
  fault_codes?: string[];
};


export type AxonPrincipalKind = "human" | "agent" | "service" | "device";

export type AxonPrincipalRef = {
  kind: AxonPrincipalKind;
  id: string;
};

export type AxonCapability =
  | "workspace_read" | "workspace_manage"
  | "agent_register" | "agent_manage"
  | "artifact_read" | "artifact_create" | "artifact_classify"
  | "task_read" | "task_create" | "task_assign" | "task_review"
  | "decision_propose" | "decision_accept" | "audit_read"
  | "device_enroll" | "device_telemetry_read" | "device_telemetry_write"
  | "device_command_propose";

/** Authorization grant only; never use this as evidence that a caller is authenticated. */
export type AxonCapabilityGrant = {
  grant_id: string;
  subject: AxonPrincipalRef;
  workspace_id: string;
  capabilities: AxonCapability[];
  issued_by: AxonPrincipalRef;
  issued_at: string;
  expires_at: string | null;
  revoked_at: string | null;
};


/** Process-local audit event. Actor is explicitly unattributed until auth is integrated. */
export type AxonAuditEvent = {
  sequence: number;
  event_id: string;
  workspace_id: string | null;
  actor: "system:local-development-unattributed" | string;
  event_type: string;
  entity_type: string;
  entity_id: string;
  occurred_at: string;
  metadata: Record<string, unknown>;
};
