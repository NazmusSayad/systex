import { invoke } from "@tauri-apps/api/core"
import { useEffect, useState } from "react"

import type { ContextSnapshot, ElementInfo, ProviderInfo, Rect } from "./types"

const PROVIDERS = ["mock", "system"]
const INTERVALS = [200, 500, 1000, 2000]

export function App() {
  const [provider, setProvider] = useState<ProviderInfo | null>(null)
  const [snapshot, setSnapshot] = useState<ContextSnapshot | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [live, setLive] = useState(true)
  const [interval, setInterval] = useState(500)
  const [showRaw, setShowRaw] = useState(false)

  useEffect(() => {
    invoke<ProviderInfo>("provider_info").then(setProvider)
  }, [])

  useEffect(() => {
    if (!live) {
      return
    }

    let cancelled = false

    async function tick() {
      try {
        const next = await invoke<ContextSnapshot>("capture")

        if (!cancelled) {
          setSnapshot(next)
          setError(null)
        }
      } catch (reason) {
        if (!cancelled) {
          setError(String(reason))
        }
      }
    }

    tick()
    const timer = window.setInterval(tick, interval)

    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [live, interval, provider?.name])

  async function captureOnce() {
    try {
      setSnapshot(await invoke<ContextSnapshot>("capture"))
      setError(null)
    } catch (reason) {
      setError(String(reason))
    }
  }

  async function switchProvider(name: string) {
    try {
      setProvider(await invoke<ProviderInfo>("set_provider", { name }))
      setError(null)
    } catch (reason) {
      setError(String(reason))
    }
  }

  return (
    <div className="app">
      <header className="bar">
        <div className="brand">
          <span className="dot" data-live={live} />
          <strong>Systex View</strong>
        </div>

        <div className="controls">
          <select value={provider?.name ?? "mock"} onChange={(e) => switchProvider(e.target.value)}>
            {PROVIDERS.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>

          <select value={interval} onChange={(e) => setInterval(Number(e.target.value))}>
            {INTERVALS.map((ms) => (
              <option key={ms} value={ms}>
                every {ms}ms
              </option>
            ))}
          </select>

          <button onClick={() => setLive(!live)}>{live ? "Pause" : "Go live"}</button>
          <button onClick={captureOnce}>Capture once</button>
          <button onClick={() => setShowRaw(!showRaw)}>{showRaw ? "Hide JSON" : "Show JSON"}</button>
        </div>
      </header>

      {provider && !provider.available && (
        <p className="note">
          Provider <code>{provider.name}</code> reports itself as unavailable — capture will fail
          until it is implemented.
        </p>
      )}

      {error && <p className="error">{error}</p>}

      {!snapshot && !error && <p className="note">No snapshot captured yet.</p>}

      {snapshot && (
        <main className="grid">
          <section className="card">
            <h2>Snapshot</h2>
            <Field label="provider" value={snapshot.provider} />
            <Field label="captured at" value={new Date(snapshot.captured_at_ms).toISOString()} />
          </section>

          <section className="card">
            <h2>Focused app</h2>
            {!snapshot.focused_app && <Empty />}
            {snapshot.focused_app && (
              <>
                <Field label="name" value={snapshot.focused_app.name} />
                <Field label="bundle id" value={snapshot.focused_app.bundle_id} />
                <Field label="pid" value={snapshot.focused_app.pid} />
              </>
            )}
          </section>

          <section className="card">
            <h2>Focused window</h2>
            {!snapshot.focused_window && <Empty />}
            {snapshot.focused_window && (
              <>
                <Field label="title" value={snapshot.focused_window.title} />
                <Field label="bounds" value={formatRect(snapshot.focused_window.bounds)} />
              </>
            )}
          </section>

          <section className="card wide">
            <h2>Caret</h2>
            {!snapshot.caret && <Empty />}
            {snapshot.caret && (
              <>
                <p className="text">
                  <span className="before">{snapshot.caret.text_before}</span>
                  <span className="caret" />
                  {snapshot.caret.selected_text && (
                    <span className="selection">{snapshot.caret.selected_text}</span>
                  )}
                  <span className="after">{snapshot.caret.text_after}</span>
                </p>
                <Field label="line" value={snapshot.caret.line} />
                <Field label="column" value={snapshot.caret.column} />
                <Field label="bounds" value={formatRect(snapshot.caret.bounds)} />
                <Element element={snapshot.caret.element} />
              </>
            )}
          </section>

          <section className="card wide">
            <h2>Pointer</h2>
            {!snapshot.pointer && <Empty />}
            {snapshot.pointer && (
              <>
                <Field
                  label="position"
                  value={`${Math.round(snapshot.pointer.position.x)}, ${Math.round(snapshot.pointer.position.y)}`}
                />
                <Field label="app" value={snapshot.pointer.app?.name ?? null} />
                <Field label="window" value={snapshot.pointer.window?.title ?? null} />
                <Element element={snapshot.pointer.element} />
              </>
            )}
          </section>

          {showRaw && (
            <section className="card wide">
              <h2>Raw</h2>
              <pre>{JSON.stringify(snapshot, null, 2)}</pre>
            </section>
          )}
        </main>
      )}
    </div>
  )
}

function Field({ label, value }: { label: string; value: string | number | null }) {
  return (
    <div className="field">
      <span className="key">{label}</span>
      <span className="value">{value === null ? "—" : value}</span>
    </div>
  )
}

function Element({ element }: { element: ElementInfo | null }) {
  if (!element) {
    return <Field label="element" value={null} />
  }

  return (
    <div className="element">
      <Field label="role" value={element.role} />
      <Field label="label" value={element.label} />
      <Field label="value" value={element.value} />
      <Field label="editable" value={element.editable ? "yes" : "no"} />
      <Field label="bounds" value={formatRect(element.bounds)} />
    </div>
  )
}

function Empty() {
  return <p className="note">Not captured.</p>
}

function formatRect(rect: Rect | null) {
  if (!rect) {
    return null
  }

  return `${Math.round(rect.x)}, ${Math.round(rect.y)} · ${Math.round(rect.width)}×${Math.round(rect.height)}`
}
