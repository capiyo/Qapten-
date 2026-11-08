/**
 * Language-neutral API contract types for the Qapten web and mobile clients.
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
