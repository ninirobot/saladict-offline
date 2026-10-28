export interface DictHit {
  id: string
  name: string
  key: string
  html: string
}

export interface DictError {
  id: string
  name: string
  message: string
}

export interface DictLookup {
  hits: DictHit[]
  errors: DictError[]
  suggestions: string[]
}

export interface LookupResponse {
  word: string
  elapsedMs: number
  local: DictLookup
}

export interface DictInfo {
  id: string
  name: string
  path: string
  enabled: boolean
  loaded: boolean
  entryCount: number
  size: number
  mdd: string | null
  error: string | null
}

export interface DictItem {
  id: string
  path: string
  name: string
  enabled: boolean
  mdd?: string | null
  entryCount: number
  size: number
}

export interface HistoryItem {
  word: string
  at: number
  source: string
  dict: string
}

export interface SelectionConfig {
  drag: boolean
  hotkey: boolean
  hotkeyCombo: string
  minChars: number
  maxChars: number
  pinOnOpen: boolean
}

export interface UiConfig {
  theme: 'auto' | 'light' | 'dark'
  fontScale: number
  popupWidth: number
}

export interface OnlineConfig {
  enabled: string[]
  network: boolean
  /** 可选代理，如 http://127.0.0.1:7890 */
  proxy: string
}

export interface AppConfig {
  dicts: DictItem[]
  selection: SelectionConfig
  ui: UiConfig
  online: OnlineConfig
  /** 机器翻译引擎凭据，键为 `${engineId}.${field}` */
  engineKeys: Record<string, string>
  historyLimit: number
  autostart: boolean
  onboarded: boolean
}

export interface Bootstrap {
  version: string
  config: AppConfig
  dicts: DictInfo[]
  history: HistoryItem[]
  paused: boolean
}

export interface PopupRequest {
  word: string
  source: string
  pinned: boolean
  seq: number
}

export interface FetchResult {
  status: number
  body: string
  url: string
}
