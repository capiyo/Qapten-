import { FormEvent, useEffect, useState } from "react";

type Workspace = { id: string; name: string; created_at: string };
type Health = { status: string; storage: string; authentication: string };

const api = async <T,>(path: string, init?: RequestInit): Promise<T> => {
  const response = await fetch(path, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers }
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? "Request failed");
  }
  return response.json() as Promise<T>;
};

export default function App() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [name, setName] = useState("");
  const [health, setHealth] = useState<Health | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  async function refresh() {
    try {
      const [items, status] = await Promise.all([
        api<Workspace[]>("/api/v1/workspaces"),
        api<Health>("/health")
      ]);
      setWorkspaces(items);
      setHealth(status);
      setError("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not reach the Qapten API.");
    }
  }

  useEffect(() => { void refresh(); }, []);

  async function createWorkspace(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!name.trim() || busy) return;
    setBusy(true);
    setError("");
    try {
      await api<Workspace>("/api/v1/workspaces", {
        method: "POST",
        body: JSON.stringify({ name: name.trim() })
      });
      setName("");
      await refresh();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Workspace creation failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="shell">
      <aside className="rail">
        <div className="brand-mark">Q</div>
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

        <section className="intro">
          <div>
            <span className="kicker">YOUR SHARED WORKSPACE</span>
            <h2>Give every agent the context it needs.</h2>
            <p>Capture work once. Hand it off with its evidence, decisions and history intact.</p>
          </div>
          <div className="orbit" aria-hidden="true">
            <span className="orbit-core">Q</span>
            <span className="orbit-node node-a">A</span>
            <span className="orbit-node node-b">B</span>
            <span className="orbit-node node-c">↗</span>
          </div>
        </section>

        <section className="section-heading">
          <div>
            <span className="kicker">START HERE</span>
            <h2>Projects and workspaces</h2>
          </div>
          <span className="count">{workspaces.length.toString().padStart(2, "0")} WORKSPACES</span>
        </section>

        <form className="create-card" onSubmit={createWorkspace}>
          <div className="create-icon">＋</div>
          <div className="create-copy">
            <label htmlFor="workspace-name">Create a workspace</label>
            <span>Start with a project, a problem, or a team of agents.</span>
          </div>
          <input id="workspace-name" value={name} onChange={event => setName(event.target.value)} placeholder="e.g. Funspot engineering" maxLength={120} />
          <button disabled={!name.trim() || busy}>{busy ? "Creating…" : "Create workspace"}</button>
        </form>

        {error && <div className="error" role="alert">{error} <button className="retry" onClick={() => void refresh()}>Retry</button></div>}

        <div className="workspace-grid">
          {workspaces.map(workspace => (
            <article className="workspace-card" key={workspace.id}>
              <div className="card-top"><span className="project-glyph">{workspace.name.slice(0, 1).toUpperCase()}</span><span className="arrow">↗</span></div>
              <h3>{workspace.name}</h3>
              <p>Workspace created {new Date(workspace.created_at).toLocaleDateString()}</p>
              <div className="card-foot"><span>AGENT SPACE</span><span className="quiet-dot" /> <span>Ready to configure</span></div>
            </article>
          ))}
          {workspaces.length === 0 && !error && (
            <div className="empty-state">
              <div className="empty-symbol">◎</div>
              <h3>Your first workspace starts here.</h3>
              <p>Create a space for Funspot, Qoin, or any problem where multiple agents need to work together.</p>
            </div>
          )}
        </div>
        <footer><span>QAPTEN</span><span>Context should travel with the work.</span><span>FOUNDATION BUILD · 0.1</span></footer>
        {health && health.storage !== "persistent" && (
          <p className="dev-warning">Development mode: workspace data is temporary and this API has no authentication. Do not use sensitive or production data.</p>
        )}
      </section>
    </main>
  );
}
