import { invoke } from '@tauri-apps/api/core'

import type {
  AppConfig,
  Bootstrap,
  DictInfo,
  FetchResult,
  HistoryItem,
  LookupResponse,
  PopupRequest
} from './types'

export const api = {
  bootstrap: () => invoke<Bootstrap>('cmd_bootstrap'),

  lookup: (word: string, source = 'manual') =>
    invoke<LookupResponse>('cmd_lookup', { word, source }),

  /** 走 Rust 侧请求，天然绕过 WebView 的 CORS 限制 */
  fetch: (opts: {
    url: string
    referer?: string
    method?: string
    body?: string
    headers?: Record<string, string>
  }) => invoke<FetchResult>('cmd_fetch_url', opts),

  dictAdd: (path: string) => invoke<DictInfo[]>('cmd_dict_add', { path }),
  dictRemove: (id: string) => invoke<DictInfo[]>('cmd_dict_remove', { id }),
  dictToggle: (id: string, enabled: boolean) =>
    invoke<DictInfo[]>('cmd_dict_toggle', { id, enabled }),
  dictMove: (id: string, to: number) => invoke<DictInfo[]>('cmd_dict_move', { id, to }),
  dictRefresh: (id: string) => invoke<DictInfo[]>('cmd_dict_refresh', { id }),

  configSet: (config: AppConfig) => invoke<AppConfig>('cmd_config_set', { config }),
  autostartSet: (enabled: boolean) => invoke<void>('cmd_autostart_set', { enabled }),

  historyClear: () => invoke<void>('cmd_history_clear'),
  historyDelete: (at: number) => invoke<void>('cmd_history_delete', { at }),

  popupHide: () => invoke<void>('cmd_popup_hide'),
  popupPin: (pinned: boolean) => invoke<void>('cmd_popup_pin', { pinned }),
  popupResize: (height: number) => invoke<void>('cmd_popup_resize', { height }),
  popupTake: () => invoke<PopupRequest | null>('cmd_popup_take'),
  openEntry: (word: string, source = 'manual') => invoke<void>('cmd_open_entry', { word, source }),
  openMain: () => invoke<void>('cmd_open_main'),

  /** 系统原生文件选择框（不引入 dialog 插件） */
  pickMdx: () => invoke<string | null>('cmd_pick_mdx'),
  /** 用默认浏览器打开外链 */
  openUrl: (url: string) => invoke<void>('cmd_open_url', { url }),
  /** 读取 mdd 资源，返回 base64 */
  readAsset: (dictId: string, path: string) =>
    invoke<string | null>('cmd_read_asset', { dictId, path }),
  /** 把前端异常写进应用日志 */
  log: (message: string, level: 'info' | 'warn' | 'error' = 'error') =>
    invoke<void>('cmd_log', { level, message }).catch(() => undefined)
}
