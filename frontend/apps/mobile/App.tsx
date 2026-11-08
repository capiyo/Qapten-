import { useEffect, useState } from "react";
import {
  ActivityIndicator,
  Pressable,
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View
} from "react-native";
import { StatusBar } from "expo-status-bar";

type Workspace = { id: string; name: string; created_at: string };
type Agent = { id: string; display_name: string; provider: string };
type Artifact = { id: string; title: string; content: string };
type Task = { id: string; title: string; objective: string; status: string; version: number };
type Health = { status: string; storage: string; authentication: string };

const API_BASE_URL = process.env.EXPO_PUBLIC_QAPTEN_API_URL ?? "http://127.0.0.1:8080";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers }
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? "Request failed");
  }
  return response.json() as Promise<T>;
}

function ActionButton({ label, onPress, disabled = false, secondary = false }: {
  label: string;
  onPress: () => void;
  disabled?: boolean;
  secondary?: boolean;
}) {
  return (
    <Pressable onPress={onPress} disabled={disabled} style={[styles.button, secondary && styles.secondaryButton, disabled && styles.disabled]}>
      <Text style={[styles.buttonText, secondary && styles.secondaryButtonText]}>{label}</Text>
    </Pressable>
  );
}

export default function App() {
  const [health, setHealth] = useState<Health | null>(null);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [selected, setSelected] = useState<Workspace | null>(null);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [workspaceName, setWorkspaceName] = useState("");
  const [agentName, setAgentName] = useState("");
  const [provider, setProvider] = useState("claude");
  const [artifactTitle, setArtifactTitle] = useState("");
  const [artifactContent, setArtifactContent] = useState("");
  const [taskTitle, setTaskTitle] = useState("");
  const [taskObjective, setTaskObjective] = useState("");
  const [assignee, setAssignee] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  async function refreshHome() {
    const [status, items] = await Promise.all([
      request<Health>("/health"),
      request<Workspace[]>("/api/v1/workspaces")
    ]);
    setHealth(status);
    setWorkspaces(items);
  }

  async function refreshWorkspace(workspace: Workspace) {
    const [agentItems, artifactItems, taskItems] = await Promise.all([
      request<Agent[]>(`/api/v1/workspaces/${workspace.id}/agents`),
      request<Artifact[]>(`/api/v1/workspaces/${workspace.id}/artifacts`),
      request<Task[]>(`/api/v1/workspaces/${workspace.id}/tasks`)
    ]);
    setAgents(agentItems);
    setArtifacts(artifactItems);
    setTasks(taskItems);
    if (!agentItems.some(agent => agent.id === assignee)) setAssignee(agentItems[0]?.id ?? "");
  }

  useEffect(() => {
    refreshHome().catch(cause => setError(cause instanceof Error ? cause.message : "API unavailable"));
  }, []);

  useEffect(() => {
    if (selected) {
      refreshWorkspace(selected).catch(cause => setError(cause instanceof Error ? cause.message : "Could not load workspace"));
    }
  }, [selected?.id]);

  async function perform(action: () => Promise<void>) {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The request failed");
    } finally {
      setBusy(false);
    }
  }

  function createWorkspace() {
    if (!workspaceName.trim()) return;
    void perform(async () => {
      const created = await request<Workspace>("/api/v1/workspaces", {
        method: "POST",
        body: JSON.stringify({ name: workspaceName.trim() })
      });
      setWorkspaceName("");
      await refreshHome();
      setSelected(created);
    });
  }

  function registerAgent() {
    if (!selected || !agentName.trim()) return;
    void perform(async () => {
      await request<Agent>(`/api/v1/workspaces/${selected.id}/agents`, {
        method: "POST",
        body: JSON.stringify({ display_name: agentName.trim(), provider })
      });
      setAgentName("");
      await refreshWorkspace(selected);
    });
  }

  function captureArtifact() {
    if (!selected || !artifactTitle.trim() || !artifactContent.trim()) return;
    void perform(async () => {
      await request<Artifact>(`/api/v1/workspaces/${selected.id}/artifacts`, {
        method: "POST",
        body: JSON.stringify({ title: artifactTitle.trim(), content: artifactContent.trim(), source_agent_id: null })
      });
      setArtifactTitle("");
      setArtifactContent("");
      await refreshWorkspace(selected);
    });
  }

  function createTask() {
    if (!selected || !assignee || !taskTitle.trim() || !taskObjective.trim()) return;
    void perform(async () => {
      await request<Task>(`/api/v1/workspaces/${selected.id}/tasks`, {
        method: "POST",
        body: JSON.stringify({
          title: taskTitle.trim(),
          objective: taskObjective.trim(),
          assignee_agent_id: assignee,
          context_artifact_ids: []
        })
      });
      setTaskTitle("");
      setTaskObjective("");
      await refreshWorkspace(selected);
    });
  }

  return (
    <SafeAreaView style={styles.safe}>
      <StatusBar style="dark" />
      <ScrollView contentContainerStyle={styles.page} keyboardShouldPersistTaps="handled">
        <View style={styles.brandRow}>
          <View style={styles.logo}><Text style={styles.logoText}>Q</Text></View>
          <Text style={styles.brand}>QAPTEN</Text>
          <View style={styles.version}><Text style={styles.versionText}>0.1</Text></View>
        </View>

        {selected ? (
          <>
            <Pressable onPress={() => setSelected(null)}><Text style={styles.back}>← All workspaces</Text></Pressable>
            <Text style={styles.eyebrow}>PROJECT WORKSPACE</Text>
            <Text style={styles.title}>{selected.name}</Text>
            <Text style={styles.description}>Register participants, capture their work and create explicit handoffs.</Text>

            <View style={styles.panel}>
              <Text style={styles.panelTitle}>01 · Agents ({agents.length})</Text>
              <TextInput value={agentName} onChangeText={setAgentName} placeholder="Agent name" style={styles.input} />
              <TextInput value={provider} onChangeText={setProvider} placeholder="Provider (claude, chatgpt…)" style={styles.input} autoCapitalize="none" />
              <ActionButton label="Register agent" onPress={registerAgent} disabled={!agentName.trim() || busy} />
              {agents.map(agent => <View key={agent.id} style={styles.listRow}><Text style={styles.rowTitle}>{agent.display_name}</Text><Text style={styles.rowMeta}>{agent.provider} · manual</Text></View>)}
              {agents.length === 0 && <Text style={styles.muted}>No agents registered yet.</Text>}
            </View>

            <View style={styles.panel}>
              <Text style={styles.panelTitle}>02 · Capture context ({artifacts.length})</Text>
              <TextInput value={artifactTitle} onChangeText={setArtifactTitle} placeholder="Artifact title" style={styles.input} />
              <TextInput value={artifactContent} onChangeText={setArtifactContent} placeholder="Paste findings, responses or project context…" style={[styles.input, styles.multiline]} multiline textAlignVertical="top" />
              <ActionButton label="Capture artifact" onPress={captureArtifact} disabled={!artifactTitle.trim() || !artifactContent.trim() || busy} />
              {artifacts.map(artifact => <View key={artifact.id} style={styles.listRow}><Text style={styles.rowTitle}>{artifact.title}</Text><Text style={styles.rowMeta} numberOfLines={2}>{artifact.content}</Text></View>)}
              {artifacts.length === 0 && <Text style={styles.muted}>No captured context yet.</Text>}
            </View>

            <View style={styles.panel}>
              <Text style={styles.panelTitle}>03 · Create handoff</Text>
              <TextInput value={taskTitle} onChangeText={setTaskTitle} placeholder="Task title" style={styles.input} />
              <TextInput value={taskObjective} onChangeText={setTaskObjective} placeholder="Expected outcome and instructions…" style={[styles.input, styles.multiline]} multiline textAlignVertical="top" />
              <Text style={styles.fieldLabel}>ASSIGN TO</Text>
              {agents.map(agent => (
                <Pressable key={agent.id} onPress={() => setAssignee(agent.id)} style={[styles.agentChoice, assignee === agent.id && styles.agentChoiceActive]}>
                  <Text style={styles.rowTitle}>{agent.display_name}</Text><Text style={styles.rowMeta}>{agent.provider}</Text>
                </Pressable>
              ))}
              <ActionButton label="Create handoff" onPress={createTask} disabled={!taskTitle.trim() || !taskObjective.trim() || !assignee || busy} />
              {tasks.map(task => <View key={task.id} style={styles.listRow}><Text style={styles.rowTitle}>{task.title}</Text><Text style={styles.rowMeta}>{task.status.replaceAll("_", " ")} · version {task.version}</Text><Text style={styles.rowMeta}>{task.objective}</Text></View>)}
              {tasks.length === 0 && <Text style={styles.muted}>No handoffs yet.</Text>}
            </View>
          </>
        ) : (
          <>
            <Text style={styles.eyebrow}>THE AGENT COLLABORATION LAYER</Text>
            <Text style={styles.title}>Work, connected.</Text>
            <Text style={styles.description}>A shared working layer for agents to exchange context, evidence, decisions and tasks—with you in control.</Text>
            <View style={styles.panel}>
              <Text style={styles.panelTitle}>SYSTEM STATUS</Text>
              <View style={styles.statusRow}>
                <View style={[styles.dot, health?.status === "ok" && styles.online]} />
                <Text style={styles.statusText}>{health ? "API connected" : "Connecting…"}</Text>
                {busy && <ActivityIndicator size="small" />}
              </View>
              <Text style={styles.rowMeta}>{health?.storage.replaceAll("_", " ") ?? "Waiting for API"}</Text>
            </View>
            <View style={styles.panel}>
              <Text style={styles.panelTitle}>YOUR WORKSPACES ({workspaces.length})</Text>
              <TextInput value={workspaceName} onChangeText={setWorkspaceName} placeholder="New workspace name" style={styles.input} />
              <ActionButton label="Create workspace" onPress={createWorkspace} disabled={!workspaceName.trim() || busy} />
              {workspaces.map(workspace => (
                <Pressable key={workspace.id} onPress={() => setSelected(workspace)} style={styles.workspaceRow}>
                  <View style={styles.workspaceGlyph}><Text style={styles.glyphText}>{workspace.name.slice(0, 1).toUpperCase()}</Text></View>
                  <View style={styles.workspaceCopy}><Text style={styles.rowTitle}>{workspace.name}</Text><Text style={styles.rowMeta}>Open project workspace</Text></View>
                  <Text style={styles.chevron}>›</Text>
                </Pressable>
              ))}
              {workspaces.length === 0 && <Text style={styles.muted}>Create a workspace for Funspot, Qoin, or a new project.</Text>}
            </View>
          </>
        )}

        {error ? <View style={styles.error}><Text style={styles.errorText}>{error}</Text></View> : null}
        <Text style={styles.footer}>CONTEXT SHOULD TRAVEL WITH THE WORK.</Text>
        <Text style={styles.warning}>Development mode: data is temporary and authentication is not implemented. Do not use sensitive or production data.</Text>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safe: { flex: 1, backgroundColor: "#f7f8fa" },
  page: { padding: 22, paddingTop: 26, paddingBottom: 48 },
  brandRow: { flexDirection: "row", alignItems: "center", marginBottom: 42 },
  logo: { width: 38, height: 38, borderRadius: 13, backgroundColor: "#1d6be5", alignItems: "center", justifyContent: "center" },
  logoText: { color: "#fff", fontSize: 21, fontWeight: "800" },
  brand: { marginLeft: 11, color: "#17212b", fontSize: 12, fontWeight: "800", letterSpacing: 2 },
  version: { marginLeft: "auto", backgroundColor: "#eaf1fc", borderRadius: 20, paddingHorizontal: 10, paddingVertical: 5 },
  versionText: { color: "#4777b6", fontSize: 10, fontWeight: "700" },
  eyebrow: { color: "#4777b6", fontSize: 10, letterSpacing: 1.5, fontWeight: "700" },
  title: { color: "#17212b", fontSize: 34, fontWeight: "800", letterSpacing: -1.2, marginTop: 12 },
  description: { color: "#697988", fontSize: 14, lineHeight: 22, marginTop: 12, marginBottom: 22 },
  panel: { backgroundColor: "#fff", borderRadius: 17, padding: 17, marginTop: 14, borderWidth: 1, borderColor: "#e9edf0" },
  panelTitle: { color: "#263746", fontSize: 13, fontWeight: "800", marginBottom: 13 },
  statusRow: { flexDirection: "row", alignItems: "center", gap: 9, marginBottom: 8 },
  dot: { width: 8, height: 8, borderRadius: 4, backgroundColor: "#d5a04c" },
  online: { backgroundColor: "#23ad78" },
  statusText: { color: "#263746", fontSize: 12, fontWeight: "700", flex: 1 },
  input: { borderWidth: 1, borderColor: "#e8edf1", borderRadius: 10, paddingHorizontal: 12, paddingVertical: 11, color: "#263746", fontSize: 12, marginBottom: 9, backgroundColor: "#fff" },
  multiline: { minHeight: 86, textAlignVertical: "top" },
  button: { borderRadius: 10, backgroundColor: "#1d6be5", paddingVertical: 12, paddingHorizontal: 14, alignItems: "center", marginVertical: 5 },
  buttonText: { color: "#fff", fontSize: 11, fontWeight: "800" },
  secondaryButton: { backgroundColor: "#eef4ff" },
  secondaryButtonText: { color: "#1d6be5" },
  disabled: { opacity: 0.45 },
  listRow: { borderTopWidth: 1, borderTopColor: "#edf0f3", paddingVertical: 11, marginTop: 7 },
  rowTitle: { color: "#344656", fontSize: 12, fontWeight: "700" },
  rowMeta: { color: "#8995a0", fontSize: 10, lineHeight: 15, marginTop: 4 },
  muted: { color: "#98a3ae", fontSize: 11, lineHeight: 17, marginTop: 10 },
  fieldLabel: { color: "#8995a0", fontSize: 9, fontWeight: "800", letterSpacing: 1, marginBottom: 8 },
  agentChoice: { borderWidth: 1, borderColor: "#edf0f3", borderRadius: 10, padding: 10, marginBottom: 7 },
  agentChoiceActive: { borderColor: "#83aef0", backgroundColor: "#f3f7ff" },
  workspaceRow: { flexDirection: "row", alignItems: "center", paddingVertical: 12, borderTopWidth: 1, borderTopColor: "#edf0f3", marginTop: 8 },
  workspaceGlyph: { width: 34, height: 34, borderRadius: 12, backgroundColor: "#eaf1fc", alignItems: "center", justifyContent: "center" },
  glyphText: { color: "#286bc7", fontSize: 13, fontWeight: "800" },
  workspaceCopy: { flex: 1, marginLeft: 11 },
  chevron: { color: "#9aa4ae", fontSize: 24 },
  back: { color: "#4777b6", fontSize: 12, marginBottom: 20 },
  error: { backgroundColor: "#fff0f0", borderRadius: 10, padding: 12, marginTop: 14 },
  errorText: { color: "#a33b3b", fontSize: 11 },
  footer: { color: "#a5afb8", fontSize: 9, letterSpacing: 1.1, marginTop: 34, textAlign: "center" },
  warning: { color: "#9a6b26", fontSize: 10, lineHeight: 15, marginTop: 10, textAlign: "center" }
});
