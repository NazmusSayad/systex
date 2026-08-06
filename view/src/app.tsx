import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { type ReactNode, useEffect, useState } from "react"

import type {
  CaretContext,
  ContextSnapshot,
  ElementInfo,
  Rect,
  Settings,
  TextNode,
  WordBox,
} from "./types"

export function App() {
  const [settings, setSettings] = useState<Settings | null>(null)
  const [snapshot, setSnapshot] = useState<ContextSnapshot | null>(null)
  const [extras, setExtras] = useState<{ window_tree: TextNode | null; words: WordBox[] }>({
    window_tree: null,
    words: [],
  })
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    invoke<Settings>("settings").then(setSettings)

    const unlisten = Promise.all([
      listen<Settings>("settings", (event) => setSettings(event.payload)),
      listen<ContextSnapshot>("context", (event) => setSnapshot(event.payload)),
    ])

    return () => {
      unlisten.then((stops) => stops.forEach((stop) => stop()))
    }
  }, [])

  useEffect(() => {
    if (!settings || (settings.route !== "window_content" && !settings.words)) {
      setExtras({ window_tree: null, words: [] })
      return
    }

    let cancelled = false

    // The window scrape and the per-word rectangles are far too slow for the live feed, so they are
    // pulled on their own schedule and merged into whatever the engine last reported.
    async function pull() {
      try {
        const next = await invoke<ContextSnapshot>("capture")

        if (!cancelled) {
          setExtras({ window_tree: next.window_tree, words: next.words })
          setError(null)
        }
      } catch (reason) {
        if (!cancelled) {
          setError(String(reason))
        }
      }
    }

    pull()
    const timer = window.setInterval(pull, Math.max(settings.interval_ms, 500))

    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [settings?.interval_ms, settings?.route, settings?.words])

  if (!settings) {
    return null
  }

  const app = snapshot?.focused_app ?? null
  const window_ = snapshot?.focused_window ?? null
  const caret = snapshot?.caret ?? null
  const element = caret?.element ?? null
  const related = snapshot?.related ?? null
  const pointer = snapshot?.pointer ?? null

  return (
    <div className="overlay" style={{ opacity: settings.opacity / 100 }}>
      <div className="backdrop" />

      {window_?.bounds && <div className="frame window" style={toStyle(window_.bounds)} />}

      {element?.bounds && <div className="frame element" style={toStyle(element.bounds)} />}

      {caret?.bounds && <div className="frame caret" style={toStyle(caret.bounds)} />}

      {pointer?.element?.bounds && (
        <div className="frame hit" style={toStyle(pointer.element.bounds)} />
      )}

      {extras.words.map((word, index) => (
        <div key={`${word.text}-${index}`} className="frame word" style={toStyle(word.rect)} />
      ))}

      {pointer && (
        <div className="pointer" style={{ left: pointer.position.x, top: pointer.position.y }} />
      )}

      <div className="readout">
        <header>
          <span className="dot" />
          <strong>systex</strong>
          <span className="meta">{settings.route === "basic" ? "basic" : "window content"}</span>
          <span className="meta">{snapshot?.provider ?? "waiting"}</span>
          <span className="meta">every {settings.interval_ms}ms</span>
          <span className="meta">
            {snapshot ? new Date(snapshot.captured_at_ms).toLocaleTimeString() : "—"}
          </span>
          {error && <span className="error">{error}</span>}
          {!snapshot && !error && <span className="meta">waiting for the accessibility engine</span>}
        </header>

        <div className="panels">
          {settings.route === "basic" && (
            <div className="cards">
              <Card title="application">
                <Row label="name" value={app?.name ?? null} />
                <Row label="bundle" value={app?.bundle_id ?? null} />
                <Row label="pid" value={app ? String(app.pid) : null} />
                <Row label="path" value={app?.path ?? null} />
              </Card>

              <Card title="window">
                <Row label="title" value={window_?.title ?? null} />
                <Row label="document" value={window_?.document ?? null} />
                <Row label="bounds" value={formatRect(window_?.bounds ?? null)} />
                <Row
                  label="state"
                  value={window_ ? `${window_.main ? "main" : "secondary"}${window_.minimized ? ", minimized" : ""}` : null}
                />
              </Card>

              <Card title="element">
                <Row label="role" value={formatElement(element)} />
                <Row label="description" value={element?.role_description ?? null} />
                <Row label="identifier" value={element?.identifier ?? null} />
                <Row label="placeholder" value={element?.placeholder ?? null} />
                <Row label="help" value={element?.help ?? null} />
                <Row
                  label="state"
                  value={
                    element ? `${element.editable ? "editable" : "read-only"}, ${element.enabled ? "enabled" : "disabled"}` : null
                  }
                />
                <Row label="characters" value={element?.character_count?.toString() ?? null} />
                <Row label="bounds" value={formatRect(element?.bounds ?? null)} />
              </Card>

              <Card title="caret">
                <Row label="position" value={formatCaret(caret)} />
                <Row label="selection" value={formatSelection(caret)} />
                <Row label="bounds" value={formatRect(caret?.bounds ?? null)} />

                {caret && hasText(caret) && (
                  <p className="text">
                    <span className="before">{tail(caret.text_before, 400)}</span>
                    <span className="marker" />
                    {caret.selected_text && <span className="selection">{caret.selected_text}</span>}
                    <span className="after">{head(caret.text_after, 400)}</span>
                  </p>
                )}
              </Card>

              <Card title="related">
                <Row label="word" value={related?.word ?? null} />
                <Row label="line" value={related?.line || null} />
                <Row label="sentence" value={related?.sentence ?? null} />

                {related?.paragraph && <p className="text">{head(related.paragraph, 900)}</p>}
              </Card>

              <Card title="pointer">
                <Row
                  label="position"
                  value={pointer ? `${Math.round(pointer.position.x)}, ${Math.round(pointer.position.y)}` : null}
                />
                <Row label="element" value={formatElement(pointer?.element ?? null)} />
                <Row label="value" value={pointer?.element?.value ?? null} />
                <Row label="app" value={pointer?.app?.name ?? null} />
                <Row label="window" value={pointer?.window?.title ?? null} />
              </Card>

              {extras.words.length > 0 && (
                <Card title={`words · ${extras.words.length}`}>
                  <p className="attributes">{extras.words.map((word) => word.text).join(" · ")}</p>
                </Card>
              )}

              {element && element.attributes.length > 0 && (
                <Card title="attributes" wide>
                  <p className="attributes">{element.attributes.join(" · ")}</p>
                  {element.parameterized_attributes.length > 0 && (
                    <p className="attributes">{element.parameterized_attributes.join(" · ")}</p>
                  )}
                </Card>
              )}
            </div>
          )}

          {settings.route === "window_content" && (
            <Card title={window_?.title ?? "window content"} tall>
              <div className="tree" style={{ columnCount: settings.columns }}>
                {!extras.window_tree && (
                  <p className="empty">no readable text in the focused window</p>
                )}

                {extras.window_tree?.children.map((child, index) => (
                  <Branch key={index} node={child} />
                ))}
              </div>
            </Card>
          )}
        </div>
      </div>
    </div>
  )
}

function Branch({ node }: { node: TextNode }) {
  return (
    <div className="branch">
      {node.text && (
        <p>
          <span className="role">{node.role.replace(/^AX/, "")}</span>
          {node.text}
        </p>
      )}

      {node.children.map((child, index) => (
        <Branch key={index} node={child} />
      ))}
    </div>
  )
}

function Card({
  title,
  wide,
  tall,
  children,
}: {
  title: string
  wide?: boolean
  tall?: boolean
  children: ReactNode
}) {
  return (
    <section className={cn("card", wide && "wide", tall && "tall")}>
      <h2>{title}</h2>
      <div className="body">{children}</div>
    </section>
  )
}

function cn(...classes: (string | false | undefined)[]) {
  return classes.filter(Boolean).join(" ")
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

function head(text: string, limit: number) {
  if (text.length <= limit) {
    return text
  }

  return `${text.slice(0, limit)}…`
}

function tail(text: string, limit: number) {
  if (text.length <= limit) {
    return text
  }

  return `…${text.slice(text.length - limit)}`
}

function toStyle(rect: Rect) {
  return { left: rect.x, top: rect.y, width: rect.width, height: rect.height }
}

function formatRect(rect: Rect | null) {
  if (!rect) {
    return null
  }

  return `${Math.round(rect.x)}, ${Math.round(rect.y)} · ${Math.round(rect.width)} × ${Math.round(rect.height)}`
}

function formatCaret(caret: CaretContext | null) {
  if (!caret) {
    return null
  }

  if (caret.line === null || caret.column === null) {
    return "unknown position"
  }

  return `line ${caret.line}, column ${caret.column}`
}

function formatSelection(caret: CaretContext | null) {
  if (!caret) {
    return null
  }

  if (caret.selection_length === 0) {
    return `offset ${caret.selection_start}`
  }

  return `${caret.selection_start} + ${caret.selection_length}`
}

function formatElement(element: ElementInfo | null) {
  if (!element) {
    return null
  }

  const role = element.subrole ? `${element.role}/${element.subrole}` : element.role

  if (!element.label) {
    return role
  }

  return `${role} · ${element.label}`
}
