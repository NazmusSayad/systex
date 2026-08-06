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
  path: string | null
  pid: number
}

export type WindowInfo = {
  title: string | null
  document: string | null
  bounds: Rect | null
  minimized: boolean
  main: boolean
}

export type ElementInfo = {
  role: string
  subrole: string | null
  role_description: string | null
  label: string | null
  help: string | null
  placeholder: string | null
  identifier: string | null
  value: string | null
  bounds: Rect | null
  editable: boolean
  enabled: boolean
  character_count: number | null
  attributes: string[]
  parameterized_attributes: string[]
}

export type CaretContext = {
  element: ElementInfo | null
  text_before: string
  text_after: string
  selected_text: string | null
  selection_start: number
  selection_length: number
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

export type RelatedContent = {
  word: string | null
  line: string
  sentence: string | null
  paragraph: string | null
  before: string
  after: string
}

export type WordBox = {
  text: string
  start: number | null
  length: number | null
  rect: Rect
}

export type ContextSnapshot = {
  captured_at_ms: number
  provider: string
  focused_app: AppInfo | null
  focused_window: WindowInfo | null
  caret: CaretContext | null
  pointer: PointerContext | null
  related: RelatedContent | null
  window_text: string | null
  words: WordBox[]
}

export type Route = "basic" | "window_content"

export type Settings = {
  route: Route
  columns: number
  interval_ms: number
  opacity: number
  overlay_visible: boolean
  pointer: boolean
  attribute_names: boolean
  window_text: boolean
  words: boolean
}
