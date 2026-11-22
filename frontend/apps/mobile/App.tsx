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
import { axonTheme } from "../../packages/theme/src";
import { auth } from "./firebase";

import type { Agent, Artifact, Health, Task, Workspace } from "../../packages/contracts/src";


const API_BASE_URL = process.env.EXPO_PUBLIC_AXON_API_URL ?? "http://127.0.0.1:8080";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = await auth?.currentUser?.getIdToken();
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...(token ? { Authorization: `Bearer ${token}` } : {}), ...init?.headers }
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

function WorkspaceApp() {
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
  const [selectedArtifactIds, setSelectedArtifactIds] = useState<string[]>([]);
  const [responseDrafts, setResponseDrafts] = useState<Record<string, string>>({});
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
      setSelectedArtifactIds([]);
      setResponseDrafts({});
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
          context_artifact_ids: selectedArtifactIds
        })
      });
      setTaskTitle("");
      setTaskObjective("");
      setSelectedArtifactIds([]);
      await refreshWorkspace(selected);
    });
  }

  async function advanceTask(task: Task, status: string) {
    if (!selected) return;
    await perform(async () => {
      await request<Task>(`/api/v1/tasks/${task.id}/transitions`, {
        method: "POST",
        body: JSON.stringify({
          expected_version: task.version,
          status,
          note: `Moved to ${status.replaceAll("_", " ")} from the mobile workspace.`
        })
      });
      await refreshWorkspace(selected);
    });
  }

  async function submitTaskResponse(task: Task) {
    if (!selected) return;
    const content = responseDrafts[task.id]?.trim() ?? "";
    if (!content) return;
    await perform(async () => {
      await request(`/api/v1/tasks/${task.id}/submit`, {
        method: "POST",
        body: JSON.stringify({
          expected_version: task.version,
          title: `${task.title} — response`,
          content
        })
      });
      setResponseDrafts(current => ({ ...current, [task.id]: "" }));
      await refreshWorkspace(selected);
    });
  }

  const nextTaskStatus: Record<string, string> = {
    offered: "acknowledged",
    acknowledged: "in_progress",
    submitted: "review_required",
    review_required: "accepted",
    changes_requested: "in_progress",
    accepted: "completed"
  };

  return (
    <SafeAreaView style={styles.safe}>
      <StatusBar style="dark" />
      <ScrollView contentContainerStyle={styles.page} keyboardShouldPersistTaps="handled">
        <View style={styles.brandRow}>
          <View style={styles.logo}><Text style={styles.logoText}>Q</Text></View>
          <Text style={styles.brand}>AXON</Text>
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
              <Text style={styles.fieldLabel}>ATTACH CONTEXT</Text>
              {artifacts.map(artifact => (
                <Pressable key={artifact.id} onPress={() => setSelectedArtifactIds(current => current.includes(artifact.id) ? current.filter(id => id !== artifact.id) : [...current, artifact.id])} style={styles.contextChoice}>
                  <Text style={styles.rowTitle}>{selectedArtifactIds.includes(artifact.id) ? "☑" : "□"}  {artifact.title}</Text>
                </Pressable>
              ))}
              <ActionButton label="Create handoff" onPress={createTask} disabled={!taskTitle.trim() || !taskObjective.trim() || !assignee || busy} />
              {tasks.map(task => (
                <View key={task.id} style={styles.listRow}>
                  <Text style={styles.rowTitle}>{task.title}</Text>
                  <Text style={styles.rowMeta}>{task.status.replaceAll("_", " ")} · version {task.version}</Text>
                  <Text style={styles.rowMeta}>{task.objective}</Text>
                  {task.status === "in_progress" ? (
                    <>
                      <TextInput value={responseDrafts[task.id] ?? ""} onChangeText={value => setResponseDrafts(current => ({ ...current, [task.id]: value }))} placeholder="Paste the assistant response…" style={[styles.input, styles.multiline]} multiline textAlignVertical="top" />
                      <ActionButton label="Submit response" onPress={() => void submitTaskResponse(task)} disabled={!(responseDrafts[task.id] ?? "").trim() || busy} />
                    </>
                  ) : null}
                  {task.status === "review_required" ? <ActionButton label="Request changes" onPress={() => void advanceTask(task, "changes_requested")} disabled={busy} secondary /> : null}
                  {nextTaskStatus[task.status] ? <ActionButton label={`Advance to ${nextTaskStatus[task.status].replaceAll("_", " ")}`} onPress={() => void advanceTask(task, nextTaskStatus[task.status])} disabled={busy} secondary /> : null}
                  {task.response_artifact_id ? <Text style={styles.responseNote}>Response artifact linked · {task.response_artifact_id.slice(0, 8)}</Text> : null}
                </View>
              ))}
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
  safe: { flex: 1, backgroundColor: axonTheme.color.canvas },
  page: { padding: 22, paddingTop: 26, paddingBottom: 48 },
  brandRow: { flexDirection: "row", alignItems: "center", marginBottom: 42 },
  logo: { width: 38, height: 38, borderRadius: 13, backgroundColor: axonTheme.color.accent, alignItems: "center", justifyContent: "center" },
  logoText: { color: axonTheme.color.accentText, fontSize: 21, fontWeight: "800" },
  brand: { marginLeft: 11, color: axonTheme.color.text, fontSize: 12, fontWeight: "800", letterSpacing: 2 },
  version: { marginLeft: "auto", backgroundColor: axonTheme.color.surfaceRaised, borderRadius: 20, paddingHorizontal: 10, paddingVertical: 5 },
  versionText: { color: axonTheme.color.accent, fontSize: 10, fontWeight: "700" },
  eyebrow: { color: axonTheme.color.accent, fontSize: 10, letterSpacing: 1.5, fontWeight: "700" },
  title: { color: axonTheme.color.text, fontSize: 34, fontWeight: "800", letterSpacing: -1.2, marginTop: 12 },
  description: { color: axonTheme.color.textMuted, fontSize: 14, lineHeight: 22, marginTop: 12, marginBottom: 22 },
  panel: { backgroundColor: axonTheme.color.surface, borderRadius: 17, padding: 17, marginTop: 14, borderWidth: 1, borderColor: axonTheme.color.border },
  panelTitle: { color: axonTheme.color.text, fontSize: 13, fontWeight: "800", marginBottom: 13 },
  statusRow: { flexDirection: "row", alignItems: "center", gap: 9, marginBottom: 8 },
  dot: { width: 8, height: 8, borderRadius: 4, backgroundColor: axonTheme.color.warning },
  online: { backgroundColor: axonTheme.color.success },
  statusText: { color: axonTheme.color.text, fontSize: 12, fontWeight: "700", flex: 1 },
  input: { borderWidth: 1, borderColor: axonTheme.color.border, borderRadius: 10, paddingHorizontal: 12, paddingVertical: 11, color: axonTheme.color.text, fontSize: 12, marginBottom: 9, backgroundColor: axonTheme.color.surfaceInput },
  multiline: { minHeight: 86, textAlignVertical: "top" },
  button: { borderRadius: 10, backgroundColor: axonTheme.color.accent, paddingVertical: 12, paddingHorizontal: 14, alignItems: "center", marginVertical: 5 },
  buttonText: { color: axonTheme.color.accentText, fontSize: 11, fontWeight: "800" },
  secondaryButton: { backgroundColor: axonTheme.color.surfaceRaised },
  secondaryButtonText: { color: axonTheme.color.accent },
  disabled: { opacity: 0.45 },
  listRow: { borderTopWidth: 1, borderTopColor: axonTheme.color.border, paddingVertical: 11, marginTop: 7 },
  contextChoice: { paddingVertical: 7, paddingHorizontal: 9, borderRadius: 9, backgroundColor: axonTheme.color.surfaceRaised, marginBottom: 5 },
  responseNote: { color: axonTheme.color.accent, fontSize: 10, marginTop: 8 },
  rowTitle: { color: axonTheme.color.text, fontSize: 12, fontWeight: "700" },
  rowMeta: { color: axonTheme.color.textMuted, fontSize: 10, lineHeight: 15, marginTop: 4 },
  muted: { color: axonTheme.color.textSubtle, fontSize: 11, lineHeight: 17, marginTop: 10 },
  fieldLabel: { color: axonTheme.color.textMuted, fontSize: 9, fontWeight: "800", letterSpacing: 1, marginBottom: 8 },
  agentChoice: { borderWidth: 1, borderColor: axonTheme.color.border, borderRadius: 10, padding: 10, marginBottom: 7 },
  agentChoiceActive: { borderColor: axonTheme.color.accent, backgroundColor: axonTheme.color.surfaceRaised },
  workspaceRow: { flexDirection: "row", alignItems: "center", paddingVertical: 12, borderTopWidth: 1, borderTopColor: axonTheme.color.border, marginTop: 8 },
  workspaceGlyph: { width: 34, height: 34, borderRadius: 12, backgroundColor: axonTheme.color.surfaceRaised, alignItems: "center", justifyContent: "center" },
  glyphText: { color: axonTheme.color.accent, fontSize: 13, fontWeight: "800" },
  workspaceCopy: { flex: 1, marginLeft: 11 },
  chevron: { color: axonTheme.color.textSubtle, fontSize: 24 },
  back: { color: axonTheme.color.accent, fontSize: 12, marginBottom: 20 },
  error: { backgroundColor: "#43292d", borderRadius: 10, padding: 12, marginTop: 14 },
  errorText: { color: axonTheme.color.danger, fontSize: 11 },
  footer: { color: axonTheme.color.textSubtle, fontSize: 9, letterSpacing: 1.1, marginTop: 34, textAlign: "center" },
  warning: { color: axonTheme.color.warning, fontSize: 10, lineHeight: 15, marginTop: 10, textAlign: "center" }
});


function AuthGate() {
  const [user, setUser] = useState<import("firebase/auth").User | null>(null);
  const [ready, setReady] = useState(false);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [createAccount, setCreateAccount] = useState(false);
  const [authError, setAuthError] = useState("");
  const [working, setWorking] = useState(false);

  useEffect(() => {
    if (!auth) { setReady(true); return; }
    return onAuthStateChanged(auth, current => { setUser(current); setReady(true); });
  }, []);

  async function submit() {
    if (!auth) return;
    setWorking(true);
    setAuthError("");
    try {
      if (createAccount) await createUserWithEmailAndPassword(auth, email.trim(), password);
      else await signInWithEmailAndPassword(auth, email.trim(), password);
    } catch (cause) {
      setAuthError(cause instanceof Error ? cause.message : "Authentication failed.");
    } finally { setWorking(false); }
  }

  if (!ready) return <SafeAreaView style={styles.safe}><View style={styles.page}><ActivityIndicator/><Text style={styles.muted}>Restoring your secure session…</Text></View></SafeAreaView>;
  if (!auth) return <SafeAreaView style={styles.safe}><View style={styles.page}><Text style={styles.title}>Firebase configuration required</Text><Text style={styles.description}>Set the EXPO_PUBLIC_FIREBASE_* variables and restart the app.</Text></View></SafeAreaView>;
  if (user) return <><View style={{paddingTop:36,paddingHorizontal:18,backgroundColor:axonTheme.color.canvas,flexDirection:"row",justifyContent:"flex-end"}}><Text style={styles.rowMeta}>{user.email} </Text><Text onPress={()=>void signOut(auth)} style={styles.back}>Sign out</Text></View><WorkspaceApp/></>;
  return <SafeAreaView style={styles.safe}><ScrollView contentContainerStyle={styles.page} keyboardShouldPersistTaps="handled"><View style={styles.brandRow}><View style={styles.logo}><Text style={styles.logoText}>A</Text></View><Text style={styles.brand}>AXON</Text></View><Text style={styles.eyebrow}>SECURE ACCESS</Text><Text style={styles.title}>{createAccount ? "Create your account" : "Welcome back"}</Text><Text style={styles.description}>Sign in to access your personal workspace and its data.</Text><TextInput value={email} onChangeText={setEmail} placeholder="Email address" keyboardType="email-address" autoCapitalize="none" autoComplete="email" style={styles.input}/><TextInput value={password} onChangeText={setPassword} placeholder="Password" secureTextEntry autoCapitalize="none" autoComplete={createAccount ? "new-password" : "password"} style={styles.input}/><ActionButton label={working ? "Please wait…" : createAccount ? "Create account" : "Sign in"} onPress={()=>void submit()} disabled={working || !email.trim() || password.length<6}/>{authError ? <Text style={styles.errorText}>{authError}</Text> : null}<Text onPress={()=>setCreateAccount(value=>!value)} style={styles.back}>{createAccount ? "Already registered? Sign in" : "New to Axon? Create an account"}</Text></ScrollView></SafeAreaView>;
}

import { onAuthStateChanged, signInWithEmailAndPassword, createUserWithEmailAndPassword, signOut } from "firebase/auth";

export default function App() { return <AuthGate />; }
