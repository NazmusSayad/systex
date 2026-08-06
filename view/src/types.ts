export type Point = {
  x: number
  y: number
}

export type Rect = {
  x: number
  y: number
  width: number
  height: number
}

export type AppInfo = {
  name: string
  bundle_id: string | null
  pid: number
}

export type WindowInfo = {
  title: string | null
  bounds: Rect | null
}

export type ElementInfo = {
  role: string
  label: string | null
  value: string | null
  bounds: Rect | null
  editable: boolean
}

export type CaretContext = {
  element: ElementInfo | null
  text_before: string
  text_after: string
  selected_text: string | null
  line: number | null
  column: number | null
  bounds: Rect | null
}

export type PointerContext = {
  position: Point
  element: ElementInfo | null
  app: AppInfo | null
  window: WindowInfo | null
}

export type ContextSnapshot = {
  captured_at_ms: number
  provider: string
  focused_app: AppInfo | null
  focused_window: WindowInfo | null
  caret: CaretContext | null
  pointer: PointerContext | null
}

export type Settings = {
  interval_ms: number
  opacity: number
  overlay_visible: boolean
}
