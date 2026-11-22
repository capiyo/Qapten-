import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { auth } from "./firebase";

import type { Agent, Artifact, Health, Task, Workspace, Chat, ChatDetail, ChatMessage, ProjectMemoryEntry } from "../../../packages/contracts/src";


const api = async <T,>(path: string, init?: RequestInit): Promise<T> => {
  const token = await auth?.currentUser?.getIdToken();
  const response = await fetch(path, {
    ...init,
    headers: { "Content-Type": "application/json", ...(token ? { Authorization: `Bearer ${token}` } : {}), ...init?.headers }
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? "Request failed");
  }
  return response.json() as Promise<T>;
};

function WorkspaceApp() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [selected, setSelected] = useState<Workspace | null>(null);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [chats, setChats] = useState<Chat[]>([]);
  const [activeChat, setActiveChat] = useState<string>("");
  const [chatDetail, setChatDetail] = useState<ChatDetail | null>(null);
  const [chatTitle, setChatTitle] = useState("");
  const [messageDraft, setMessageDraft] = useState("");
  const [memoryDraft, setMemoryDraft] = useState("");
  const [memoryType, setMemoryType] = useState<ProjectMemoryEntry["type"]>("note");
  const [editingMemoryId, setEditingMemoryId] = useState("");
  const [editMemoryDraft, setEditMemoryDraft] = useState("");
  const [name, setName] = useState("");
  const [agentName, setAgentName] = useState("");
  const [provider, setProvider] = useState("claude");
  const [artifactTitle, setArtifactTitle] = useState("");
  const [artifactContent, setArtifactContent] = useState("");
  const [taskTitle, setTaskTitle] = useState("");
  const [taskObjective, setTaskObjective] = useState("");
  const [assignee, setAssignee] = useState("");
  const [selectedArtifactIds, setSelectedArtifactIds] = useState<string[]>([]);
  const [responseDrafts, setResponseDrafts] = useState<Record<string, string>>({});
  const [health, setHealth] = useState<Health | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  async function refreshWorkspaces() {
    const [items, status] = await Promise.all([
      api<Workspace[]>("/api/v1/workspaces"),
      api<Health>("/health")
    ]);
    setWorkspaces(items);
    setHealth(status);
    setError("");
  }

  async function refreshWorkspace(workspace: Workspace) {
    const [agentItems, artifactItems, taskItems] = await Promise.all([
      api<Agent[]>(`/api/v1/workspaces/${workspace.id}/agents`),
      api<Artifact[]>(`/api/v1/workspaces/${workspace.id}/artifacts`),
      api<Task[]>(`/api/v1/workspaces/${workspace.id}/tasks`)
    ]);
    setAgents(agentItems);
    setArtifacts(artifactItems);
    setTasks(taskItems);
    const chatItems = await api<Chat[]>(`/api/v1/workspaces/${workspace.id}/chats`);
    setChats(chatItems);
    if (activeChat && !chatItems.some(chat => chat.id === activeChat)) {
      setActiveChat("");
      setChatDetail(null);
    }
    setAssignee(current => agentItems.some(agent => agent.id === current) ? current : agentItems[0]?.id ?? "");
  }

  useEffect(() => {
    void refreshWorkspaces().catch(cause => setError(cause instanceof Error ? cause.message : "Could not reach the Axon API."));
  }, []);

  useEffect(() => {
    if (selected) {
      setSelectedArtifactIds([]);
      setResponseDrafts({});
      void refreshWorkspace(selected).catch(cause => setError(cause instanceof Error ? cause.message : "Could not load workspace."));
    } else {
      setAgents([]);
      setArtifacts([]);
      setTasks([]);
    }
  }, [selected?.id]);

  useEffect(() => {
    if (!selected || !activeChat) { setChatDetail(null); return; }
    api<ChatDetail>(`/api/v1/workspaces/${selected.id}/chats/${activeChat}`)
      .then(setChatDetail)
      .catch(cause => setError(cause instanceof Error ? cause.message : "Could not load chat."));
  }, [selected?.id, activeChat]);

  async function createChat(event: FormEvent<HTMLFormElement>) {
    if (!selected) return;
    await runForm(event, async () => {
      const chat = await api<Chat>(`/api/v1/workspaces/${selected.id}/chats`, {
        method: "POST", body: JSON.stringify({ title: chatTitle.trim() })
      });
      setChatTitle("");
      setChats(current => [chat, ...current]);
      setActiveChat(chat.id);
    });
  }

  async function sendChatMessage(event: FormEvent<HTMLFormElement>) {
    if (!selected || !activeChat || !messageDraft.trim()) return;
    await runForm(event, async () => {
      await api<ChatMessage>(`/api/v1/workspaces/${selected.id}/chats/${activeChat}/messages`, {
        method: "POST", body: JSON.stringify({ role: "user", content: messageDraft.trim() })
      });
      setMessageDraft("");
      const [detail, chatItems] = await Promise.all([
        api<ChatDetail>(`/api/v1/workspaces/${selected.id}/chats/${activeChat}`),
        api<Chat[]>(`/api/v1/workspaces/${selected.id}/chats`)
      ]);
      setChatDetail(detail);
      setChats(chatItems);
    });
  }

  async function proposeMemory(event: FormEvent<HTMLFormElement>) {
    if (!selected || !activeChat || !memoryDraft.trim()) return;
    await runForm(event, async () => {
      await api<ProjectMemoryEntry>(`/api/v1/workspaces/${selected.id}/chats/${activeChat}/memory`, {
        method: "POST", body: JSON.stringify({ type: memoryType, text: memoryDraft.trim() })
      });
      setMemoryDraft("");
      setChatDetail(await api<ChatDetail>(`/api/v1/workspaces/${selected.id}/chats/${activeChat}`));
    });
  }

  async function saveMemoryEdit(entry: ProjectMemoryEntry) {
    if (!selected || !activeChat || !editMemoryDraft.trim()) return;
    setBusy(true);
    try {
      await api<ProjectMemoryEntry>(`/api/v1/workspaces/${selected.id}/chats/${activeChat}/memory/${entry.id}/edit`, {
        method: "POST", body: JSON.stringify({ text: editMemoryDraft.trim(), reason: entry.reason })
      });
      setEditingMemoryId("");
      setEditMemoryDraft("");
      setChatDetail(await api<ChatDetail>(`/api/v1/workspaces/${selected.id}/chats/${activeChat}`));
    } catch (cause) { setError(cause instanceof Error ? cause.message : "Memory edit failed."); }
    finally { setBusy(false); }
  }

  async function reviewMemory(entry: ProjectMemoryEntry, action: "approve" | "reject" | "delete") {
    if (!selected || !activeChat) return;
    setBusy(true);
    setError("");
    try {
      await api<ProjectMemoryEntry>(`/api/v1/workspaces/${selected.id}/chats/${activeChat}/memory/${entry.id}/${action}`, { method: "POST" });
      setChatDetail(await api<ChatDetail>(`/api/v1/workspaces/${selected.id}/chats/${activeChat}`));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Memory update failed.");
    } finally { setBusy(false); }
  }

  async function runForm(event: FormEvent<HTMLFormElement>, action: () => Promise<void>) {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The request failed.");
    } finally {
      setBusy(false);
    }
  }

  async function createWorkspace(event: FormEvent<HTMLFormElement>) {
    await runForm(event, async () => {
      const created = await api<Workspace>("/api/v1/workspaces", {
        method: "POST",
        body: JSON.stringify({ name: name.trim() })
      });
      setName("");
      await refreshWorkspaces();
      setSelected(created);
    });
  }

  async function createAgent(event: FormEvent<HTMLFormElement>) {
    if (!selected) return;
    await runForm(event, async () => {
      await api<Agent>(`/api/v1/workspaces/${selected.id}/agents`, {
        method: "POST",
        body: JSON.stringify({ display_name: agentName.trim(), provider: provider.trim() })
      });
      setAgentName("");
      await refreshWorkspace(selected);
    });
  }

  async function createArtifact(event: FormEvent<HTMLFormElement>) {
    if (!selected) return;
    await runForm(event, async () => {
      await api<Artifact>(`/api/v1/workspaces/${selected.id}/artifacts`, {
        method: "POST",
        body: JSON.stringify({ title: artifactTitle.trim(), content: artifactContent.trim(), source_agent_id: null })
      });
      setArtifactTitle("");
      setArtifactContent("");
      await refreshWorkspace(selected);
    });
  }

  async function createTask(event: FormEvent<HTMLFormElement>) {
    if (!selected || !assignee) return;
    await runForm(event, async () => {
      await api<Task>(`/api/v1/workspaces/${selected.id}/tasks`, {
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
    setBusy(true);
    setError("");
    try {
      await api<Task>(`/api/v1/tasks/${task.id}/transitions`, {
        method: "POST",
        body: JSON.stringify({
          expected_version: task.version,
          status,
          note: `Moved to ${status.replaceAll("_", " ")} from the workspace interface.`
        })
      });
      await refreshWorkspace(selected);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Task update failed.");
    } finally {
      setBusy(false);
    }
  }

  async function submitTaskResponse(task: Task) {
    if (!selected) return;
    const content = responseDrafts[task.id]?.trim() ?? "";
    if (!content || busy) return;
    setBusy(true);
    setError("");
    try {
      await api(`/api/v1/tasks/${task.id}/submit`, {
        method: "POST",
        body: JSON.stringify({
          expected_version: task.version,
          title: `${task.title} — response`,
          content
        })
      });
      setResponseDrafts(current => ({ ...current, [task.id]: "" }));
      await refreshWorkspace(selected);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Response submission failed.");
    } finally {
      setBusy(false);
    }
  }

  const nextStatus: Record<string, string> = {
    offered: "acknowledged",
    acknowledged: "in_progress",
    in_progress: "submitted",
    submitted: "review_required",
    review_required: "accepted",
    changes_requested: "in_progress",
    accepted: "completed"
  };

  return (
    <main className="shell">
      <aside className="rail">
        <div className="brand-mark">A</div>
        <span className="rail-label">WORKSPACES</span>
        <div className="rail-dot active" aria-label="Workspace area" />
        <div className="rail-bottom">0.1</div>
      </aside>
      <section className="content">
        <header className="topbar">
          <div>
            <p className="eyebrow">THE AGENT COLLABORATION LAYER</p>
            <h1>Work, connected.</h1>
          </div>
          <div className="status">
            <span className={health?.status === "ok" ? "pulse online" : "pulse"} />
            {health?.status === "ok" ? "API connected" : "API unavailable"}
          </div>
        </header>

        {!selected ? (
          <>
            <section className="intro">
              <div>
                <span className="kicker">YOUR SHARED WORKSPACE</span>
                <h2>Give every agent the context it needs.</h2>
                <p>Capture work once. Hand it off with its evidence, decisions and history intact.</p>
              </div>
              <div className="orbit" aria-hidden="true">
                <span className="orbit-core">A</span>
                <span className="orbit-node node-a">A</span>
                <span className="orbit-node node-b">B</span>
                <span className="orbit-node node-c">↗</span>
              </div>
            </section>
            <section className="section-heading">
              <div><span className="kicker">START HERE</span><h2>Projects and workspaces</h2></div>
              <span className="count">{workspaces.length.toString().padStart(2, "0")} WORKSPACES</span>
            </section>
            <form className="create-card" onSubmit={createWorkspace}>
              <div className="create-icon">＋</div>
              <div className="create-copy">
                <label htmlFor="workspace-name">Create a workspace</label>
                <span>Start with a project, a problem, or a team of agents.</span>
              </div>
              <input id="workspace-name" value={name} onChange={event => setName(event.target.value)} placeholder="e.g. Funspot engineering" maxLength={120} required />
              <button disabled={!name.trim() || busy}>{busy ? "Creating…" : "Create workspace"}</button>
            </form>
            <div className="workspace-grid">
              {workspaces.map(workspace => (
                <button className="workspace-card workspace-button" key={workspace.id} onClick={() => setSelected(workspace)}>
                  <div className="card-top"><span className="project-glyph">{workspace.name.slice(0, 1).toUpperCase()}</span><span className="arrow">↗</span></div>
                  <h3>{workspace.name}</h3>
                  <p>Created {new Date(workspace.created_at).toLocaleDateString()}</p>
                  <div className="card-foot"><span>OPEN WORKSPACE</span><span className="quiet-dot" /></div>
                </button>
              ))}
              {workspaces.length === 0 && !error && (
                <div className="empty-state">
                  <div className="empty-symbol">◎</div>
                  <h3>Your first workspace starts here.</h3>
                  <p>Create a space for Funspot, Qoin, or any problem where multiple agents need to work together.</p>
                </div>
              )}
            </div>
          </>
        ) : (
          <>
            <div className="workspace-header">
              <button className="back-link" onClick={() => setSelected(null)}>← All workspaces</button>
              <span className="count">PROJECT WORKSPACE</span>
              <h2>{selected.name}</h2>
              <p>Register agents, capture their work, and create explicit handoffs.</p>
            </div>
            <section className="panel chat-panel">
              <div className="panel-heading"><div><span className="kicker">04 · PROJECT CONVERSATIONS</span><h3>Chats & memory</h3></div><span className="count">{chats.length} CHATS</span></div>
              <div className="chat-layout">
                <aside className="chat-sidebar">
                  <form className="stack-form" onSubmit={createChat}>
                    <label htmlFor="chat-title">New chat</label>
                    <input id="chat-title" value={chatTitle} onChange={event => setChatTitle(event.target.value)} placeholder="What are we working on?" maxLength={160} required />
                    <button disabled={!chatTitle.trim() || busy}>Create chat</button>
                  </form>
                  <div className="chat-list">
                    {chats.map(chat => <button type="button" className={activeChat === chat.id ? "chat-list-item active" : "chat-list-item"} key={chat.id} onClick={() => setActiveChat(chat.id)}><strong>{chat.title}</strong><small>{new Date(chat.updated_at).toLocaleString()}</small></button>)}
                    {chats.length === 0 && <p className="muted">Chats created in this workspace will appear here.</p>}
                  </div>
                </aside>
                <div className="chat-main">
                  {!chatDetail ? <div className="empty-state compact"><h3>Select a chat or create one.</h3><p>Each conversation keeps its own messages, approved memory and resume brief.</p></div> : <>
                    <div className="resume-brief"><span className="kicker">RESUME BRIEF · APPROVED MEMORY ONLY</span><p>{chatDetail.resume_brief || "No context yet. Add messages and approve useful memory to build a concise handoff."}</p></div>
                    <div className="chat-messages">{chatDetail.messages.map(message => <article className={`chat-message message-${message.role}`} key={message.id}><div><strong>{message.role}</strong>{message.answered_by && <small> · {message.answered_by}</small>}</div><p>{message.content}</p></article>)}{chatDetail.messages.length === 0 && <p className="muted">No messages yet.</p>}</div>
                    <form className="chat-compose" onSubmit={sendChatMessage}><textarea value={messageDraft} onChange={event => setMessageDraft(event.target.value)} rows={3} maxLength={256000} placeholder="Write a message to this project chat…" required /><button disabled={!messageDraft.trim() || busy}>Send message</button></form>
                    <div className="memory-section">
                      <div className="panel-heading"><div><span className="kicker">PROJECT MEMORY</span><h3>Review proposed knowledge</h3></div></div>
                      <form className="memory-compose" onSubmit={proposeMemory}><select value={memoryType} onChange={event => setMemoryType(event.target.value as ProjectMemoryEntry["type"])}><option value="decision">Decision</option><option value="fact">Fact</option><option value="constraint">Constraint</option><option value="open_task">Open task</option><option value="note">Note</option></select><input value={memoryDraft} onChange={event => setMemoryDraft(event.target.value)} placeholder="Capture a decision, fact, constraint or open task…" maxLength={4000} required /><button disabled={!memoryDraft.trim() || busy}>Propose memory</button></form>
                      {chatDetail.memory.map(entry => <article className="memory-item" key={entry.id}><div><strong>{entry.type.replaceAll("_", " ")}</strong><small> · {entry.status}</small></div>{editingMemoryId === entry.id ? <div className="memory-compose"><input value={editMemoryDraft} onChange={event => setEditMemoryDraft(event.target.value)} maxLength={4000} /><button type="button" disabled={busy || !editMemoryDraft.trim()} onClick={() => void saveMemoryEdit(entry)}>Save</button><button type="button" className="secondary" onClick={() => setEditingMemoryId("")}>Cancel</button></div> : <p>{entry.text}</p>}<div className="memory-actions">{entry.status === "proposed" && <><button type="button" disabled={busy} onClick={() => void reviewMemory(entry, "approve")}>Approve</button><button type="button" className="secondary" disabled={busy} onClick={() => void reviewMemory(entry, "reject")}>Reject</button></>}<button type="button" className="secondary" disabled={busy} onClick={() => { setEditingMemoryId(entry.id); setEditMemoryDraft(entry.text); }}>Edit</button><button type="button" className="secondary" disabled={busy} onClick={() => void reviewMemory(entry, "delete")}>Delete</button></div></article>)}
                      {chatDetail.memory.length === 0 && <p className="muted">No project memory proposed yet. Only approved entries shape future resume briefs.</p>}
                    </div>
                  </>}
                </div>
              </div>
            </section>
            <div className="workspace-columns">
              <section className="panel">
                <div className="panel-heading"><div><span className="kicker">01 · PARTICIPANTS</span><h3>Agents</h3></div><span className="count">{agents.length}</span></div>
                <form className="stack-form" onSubmit={createAgent}>
                  <label htmlFor="agent-name">Agent name</label>
                  <input id="agent-name" value={agentName} onChange={event => setAgentName(event.target.value)} placeholder="e.g. Architecture reviewer" maxLength={100} required />
                  <label htmlFor="provider">Provider label</label>
                  <select id="provider" value={provider} onChange={event => setProvider(event.target.value)}>
                    <option value="claude">Claude</option><option value="chatgpt">ChatGPT</option><option value="gemini">Gemini</option><option value="local">Local model</option><option value="other">Other / manual</option>
                  </select>
                  <button disabled={!agentName.trim() || busy}>{busy ? "Saving…" : "Register agent"}</button>
                </form>
                <div className="item-list">
                  {agents.map(agent => <div className="agent-item" key={agent.id}><span className="agent-avatar">{agent.display_name.slice(0,1).toUpperCase()}</span><div><strong>{agent.display_name}</strong><small>{agent.provider} · manual connection</small></div></div>)}
                  {agents.length === 0 && <p className="muted">No agents yet. Register the first participant above.</p>}
                </div>
              </section>

              <section className="panel">
                <div className="panel-heading"><div><span className="kicker">02 · SHARED KNOWLEDGE</span><h3>Artifacts</h3></div><span className="count">{artifacts.length}</span></div>
                <form className="stack-form" onSubmit={createArtifact}>
                  <label htmlFor="artifact-title">Artifact title</label>
                  <input id="artifact-title" value={artifactTitle} onChange={event => setArtifactTitle(event.target.value)} placeholder="e.g. Bug investigation notes" maxLength={200} required />
                  <label htmlFor="artifact-content">Captured content</label>
                  <textarea id="artifact-content" value={artifactContent} onChange={event => setArtifactContent(event.target.value)} placeholder="Paste the relevant response, findings, or project context…" maxLength={512000} rows={4} required />
                  <button disabled={!artifactTitle.trim() || !artifactContent.trim() || busy}>{busy ? "Saving…" : "Capture artifact"}</button>
                </form>
                <div className="item-list">
                  {artifacts.map(artifact => <details className="artifact-item" key={artifact.id}><summary><strong>{artifact.title}</strong><small>{new Date(artifact.captured_at).toLocaleString()}</small></summary><p>{artifact.content}</p></details>)}
                  {artifacts.length === 0 && <p className="muted">Captured outputs and evidence will appear here.</p>}
                </div>
              </section>
            </div>

            <section className="panel task-panel">
              <div className="panel-heading"><div><span className="kicker">03 · COORDINATION</span><h3>Task handoffs</h3></div><span className="count">{tasks.length} TASKS</span></div>
              <form className="task-form" onSubmit={createTask}>
                <div><label htmlFor="task-title">Task title</label><input id="task-title" value={taskTitle} onChange={event => setTaskTitle(event.target.value)} placeholder="e.g. Review the API design" maxLength={160} required /></div>
                <div><label htmlFor="assignee">Assign to agent</label><select id="assignee" value={assignee} onChange={event => setAssignee(event.target.value)} required><option value="">Choose an agent</option>{agents.map(agent => <option value={agent.id} key={agent.id}>{agent.display_name} ({agent.provider})</option>)}</select></div>
                <div className="task-objective"><label htmlFor="task-objective">Expected outcome / instructions</label><textarea id="task-objective" value={taskObjective} onChange={event => setTaskObjective(event.target.value)} placeholder="Describe what the receiving agent should do and what evidence it should return…" maxLength={10000} rows={3} required /></div>
                <div className="context-picker"><span className="field-label">Attach context artifacts</span>{artifacts.length === 0 ? <small>No artifacts captured yet.</small> : artifacts.map(artifact => <label className="context-option" key={artifact.id}><input type="checkbox" checked={selectedArtifactIds.includes(artifact.id)} onChange={event => setSelectedArtifactIds(current => event.target.checked ? [...current, artifact.id] : current.filter(id => id !== artifact.id))} /><span>{artifact.title}</span></label>)}</div>
                <button disabled={!taskTitle.trim() || !taskObjective.trim() || !assignee || busy || agents.length === 0}>{busy ? "Creating…" : "Create handoff"}</button>
              </form>
              <div className="task-list">
                {tasks.map(task => {
                  const assigneeAgent = agents.find(agent => agent.id === task.assignee_agent_id);
                  const next = nextStatus[task.status];
                  return <article className="task-card" key={task.id}>
                    <div className="task-card-head"><div><span className="kicker">TASK · V{task.version}</span><h4>{task.title}</h4></div><span className={`task-status status-${task.status}`}>{task.status.replaceAll("_", " ")}</span></div>
                    <p>{task.objective}</p>
                    <div className="task-meta"><span>Assigned to <strong>{assigneeAgent?.display_name ?? "Unknown agent"}</strong></span><span>{task.transitions.length} events</span></div>
                    {task.status === "in_progress" && <div className="response-capture"><label htmlFor={`response-${task.id}`}>Capture the agent's response</label><textarea id={`response-${task.id}`} rows={4} value={responseDrafts[task.id] ?? ""} onChange={event => setResponseDrafts(current => ({ ...current, [task.id]: event.target.value }))} placeholder="Paste the response returned by the assigned assistant…" /><button className="advance-button" disabled={busy || !(responseDrafts[task.id] ?? "").trim()} onClick={() => void submitTaskResponse(task)}>Submit response as artifact →</button></div>}
                    {next && task.status !== "in_progress" && <button className="advance-button" disabled={busy} onClick={() => void advanceTask(task, next)}>Advance to {next.replaceAll("_", " ")} →</button>}
                    {task.status === "review_required" && <button className="request-changes-button" disabled={busy} onClick={() => void advanceTask(task, "changes_requested")}>Request changes</button>}
                    {task.response_artifact_id && <p className="response-linked">Response artifact linked · {task.response_artifact_id.slice(0, 8)}</p>}
                    <details className="history"><summary>View handoff history</summary>{task.transitions.map((transition, index) => <div className="history-row" key={index}><strong>{transition.to.replaceAll("_", " ")}</strong><span>{transition.note}</span><small>{new Date(transition.occurred_at).toLocaleString()}</small></div>)}</details>
                  </article>;
                })}
                {tasks.length === 0 && <div className="empty-state compact"><h3>No handoffs yet.</h3><p>Register an agent and create the first task above.</p></div>}
              </div>
            </section>
          </>
        )}

        {error && <div className="error" role="alert">{error} <button className="retry" onClick={() => selected ? void refreshWorkspace(selected) : void refreshWorkspaces()}>Retry</button></div>}
        <footer><span>AXON</span><span>Context should travel with the work.</span><span>FOUNDATION BUILD · 0.1</span></footer>
        {health && health.storage !== "persistent" && (
          <p className="dev-warning">Development mode: data is temporary and this API has no authentication. Do not use sensitive or production data.</p>
        )}
      </section>
    </main>
  );
}


function AuthGate() {
  const [user, setUser] = useState<import("firebase/auth").User | null>(null);
  const [ready, setReady] = useState(false);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [mode, setMode] = useState<"sign_in" | "create">("sign_in");
  const [authError, setAuthError] = useState("");
  const [working, setWorking] = useState(false);

  useEffect(() => {
    if (!auth) { setReady(true); return; }
    return onAuthStateChanged(auth, current => { setUser(current); setReady(true); });
  }, []);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!auth) return;
    setWorking(true);
    setAuthError("");
    try {
      if (mode === "create") await createUserWithEmailAndPassword(auth, email.trim(), password);
      else await signInWithEmailAndPassword(auth, email.trim(), password);
    } catch (cause) {
      setAuthError(cause instanceof Error ? cause.message : "Authentication failed.");
    } finally { setWorking(false); }
  }

  if (!ready) return <main className="shell"><section className="content"><p>Restoring your secure session…</p></section></main>;
  if (!auth) return <main className="shell"><section className="content"><h1>Firebase configuration required</h1><p>Set the VITE_FIREBASE_* environment variables for this web app, then restart it.</p></section></main>;
  if (user) return <><div style={{position:"fixed",right:24,top:18,zIndex:10,display:"flex",gap:12,alignItems:"center",fontSize:12}}><span>{user.email}</span><button onClick={() => { if (auth) void signOut(auth); }}>Sign out</button></div><WorkspaceApp /></>;
  return <main className="shell"><section className="content" style={{maxWidth:520,margin:"12vh auto"}}><p className="eyebrow">AXON · SECURE ACCESS</p><h1>{mode === "create" ? "Create your account" : "Welcome back"}</h1><p>Sign in to access your personal workspace and its data.</p><form className="stack-form" onSubmit={submit}><label htmlFor="auth-email">Email</label><input id="auth-email" type="email" autoComplete="email" required value={email} onChange={e=>setEmail(e.target.value)}/><label htmlFor="auth-password">Password</label><input id="auth-password" type="password" minLength={6} autoComplete={mode === "create" ? "new-password" : "current-password"} required value={password} onChange={e=>setPassword(e.target.value)}/><button disabled={working}>{working ? "Please wait…" : mode === "create" ? "Create account" : "Sign in"}</button></form>{authError && <p role="alert">{authError}</p>}<button className="back-link" onClick={()=>setMode(mode === "create" ? "sign_in" : "create")}>{mode === "create" ? "Already registered? Sign in" : "New to Axon? Create an account"}</button></section></main>;
}

import { onAuthStateChanged, signInWithEmailAndPassword, createUserWithEmailAndPassword, signOut } from "firebase/auth";

export default function App() { return <AuthGate />; }
