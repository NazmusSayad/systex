import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { useEffect, useState } from "react"

import type { CaretContext, ContextSnapshot, ElementInfo, Rect, Settings } from "./types"

export function App() {
  const [settings, setSettings] = useState<Settings | null>(null)
  const [snapshot, setSnapshot] = useState<ContextSnapshot | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    invoke<Settings>("settings").then(setSettings)

    const unlisten = listen<Settings>("settings", (event) => setSettings(event.payload))

    return () => {
      unlisten.then((stop) => stop())
    }
  }, [])

  useEffect(() => {
    if (!settings) {
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
    const timer = window.setInterval(tick, settings.interval_ms)

    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [settings?.interval_ms])

  if (!settings) {
    return null
  }

  return (
    <div className="overlay" style={{ opacity: settings.opacity / 100 }}>
      {snapshot?.focused_window?.bounds && (
        <div className="frame window" style={toStyle(snapshot.focused_window.bounds)} />
      )}

      {snapshot?.caret?.element?.bounds && (
        <div className="frame element" style={toStyle(snapshot.caret.element.bounds)} />
      )}

      {snapshot?.caret?.bounds && (
        <div className="frame caret" style={toStyle(snapshot.caret.bounds)} />
      )}

      {snapshot?.pointer && (
        <div
          className="pointer"
          style={{ left: snapshot.pointer.position.x, top: snapshot.pointer.position.y }}
        />
      )}

      <div className="hud">
        <header>
          <span className="dot" />
          <strong>systex</strong>
          <span className="meta">every {settings.interval_ms}ms</span>
        </header>

        {error && <p className="error">{error}</p>}

        {!error && snapshot && (
          <>
            <Row label="app" value={snapshot.focused_app?.name ?? null} />
            <Row label="window" value={snapshot.focused_window?.title ?? null} />
            <Row label="caret" value={formatCaret(snapshot)} />
            <Row label="element" value={formatElement(snapshot.caret?.element ?? null)} />
            <Row
              label="pointer"
              value={
                snapshot.pointer
                  ? `${Math.round(snapshot.pointer.position.x)}, ${Math.round(snapshot.pointer.position.y)}`
                  : null
              }
            />
            <Row label="under pointer" value={formatElement(snapshot.pointer?.element ?? null)} />

            {snapshot.caret && hasText(snapshot.caret) && (
              <p className="text">
                <span className="before">{snapshot.caret.text_before}</span>
                <span className="marker" />
                {snapshot.caret.selected_text && (
                  <span className="selection">{snapshot.caret.selected_text}</span>
                )}
                <span className="after">{snapshot.caret.text_after}</span>
              </p>
            )}
          </>
        )}
      </div>
    </div>
  )
}

function Row({ label, value }: { label: string; value: string | null }) {
  return (
    <div className="row">
      <span className="key">{label}</span>
      <span className="value">{value === null ? "—" : value}</span>
    </div>
  )
}

function hasText(caret: CaretContext) {
  return caret.text_before.length > 0 || caret.text_after.length > 0 || caret.selected_text !== null
}

function toStyle(rect: Rect) {
  return { left: rect.x, top: rect.y, width: rect.width, height: rect.height }
}

function formatCaret(snapshot: ContextSnapshot) {
  if (!snapshot.caret) {
    return null
  }

  if (snapshot.caret.line === null || snapshot.caret.column === null) {
    return "unknown position"
  }

  return `line ${snapshot.caret.line}, column ${snapshot.caret.column}`
}

function formatElement(element: ElementInfo | null) {
  if (!element) {
    return null
  }

  if (!element.label) {
    return element.role
  }

  return `${element.role} · ${element.label}`
}
