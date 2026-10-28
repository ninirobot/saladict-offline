import { listen } from '@tauri-apps/api/event'
import { useCallback, useEffect, useRef, useState } from 'preact/hooks'

import { api } from '../api'
import { emptyLookup, startLookup, type LookupState } from '../lookup'
import { applyTheme } from '../util'
import type { AppConfig, PopupRequest } from '../types'
import { ResultView } from './ResultView'
import { Icon } from './icons'

const SOURCE_LABEL: Record<string, string> = {
  drag: '划词',
  hotkey: '快捷键',
  manual: '手动',
  tray: '托盘'
}

export function Popup() {
  const [state, setState] = useState<LookupState>(emptyLookup)
  const [pinned, setPinned] = useState(false)
  const [source, setSource] = useState('drag')
  const configRef = useRef<AppConfig | null>(null)
  const cancelRef = useRef<null | (() => void)>(null)
  const lastSeq = useRef(0)
  const bodyRef = useRef<HTMLDivElement>(null)
  const shellRef = useRef<HTMLDivElement>(null)
  const headRef = useRef<HTMLDivElement>(null)
  const footRef = useRef<HTMLDivElement>(null)

  const update = useCallback((patch: Partial<LookupState>) => {
    setState(prev => ({ ...prev, ...patch }))
  }, [])

  const lookup = useCallback(
    (word: string, src: string) => {
      cancelRef.current?.()
      const config = configRef.current
      const enabled = config && config.online.network ? config.online.enabled : []
      cancelRef.current = startLookup(word, {
        enabledEngines: enabled,
        keys: config?.engineKeys ?? {},
        source: src,
        onUpdate: update
      })
      setSource(src)
    },
    [update]
  )

  useEffect(() => {
    document.body.classList.add('popup-body')
    const unlisteners: Array<() => void> = []
    let disposed = false

    const handle = (payload: PopupRequest) => {
      if (!payload.word || payload.seq <= lastSeq.current) return
      lastSeq.current = payload.seq
      setPinned(payload.pinned)
      lookup(payload.word, payload.source)
    }

    // 先拿到配置，再取回请求，保证第一次查询就能带上在线词典开关
    void api
      .bootstrap()
      .then(async boot => {
        if (disposed) return
        configRef.current = boot.config
        applyTheme(boot.config.ui.theme)
        const payload = await api.popupTake()
        if (payload && !disposed) handle(payload)
      })
      .catch(() => undefined)

    void listen<PopupRequest>('popup-show', event => handle(event.payload)).then(fn =>
      unlisteners.push(fn)
    )

    void listen<AppConfig>('config-changed', event => {
      configRef.current = event.payload
      applyTheme(event.payload.ui.theme)
    }).then(fn => unlisteners.push(fn))

    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') void api.popupHide()
    }
    window.addEventListener('keydown', onKey)

    return () => {
      disposed = true
      unlisteners.forEach(fn => fn())
      window.removeEventListener('keydown', onKey)
      cancelRef.current?.()
    }
  }, [lookup])

  // 内容变化后把真实高度回报给 Rust，浮窗才不会留一大片空白
  useEffect(() => {
    const id = requestAnimationFrame(() => {
      const shell = shellRef.current
      if (!shell) return
      const body = bodyRef.current
      const content = body ? Math.max(body.scrollHeight, body.offsetHeight) : 0
      const chrome =
        (headRef.current?.offsetHeight ?? 0) + (footRef.current?.offsetHeight ?? 0)
      const needed = Math.min(chrome + content + 14, 620)
      if (needed > 80) void api.popupResize(needed)
    })
    return () => cancelAnimationFrame(id)
  }, [state, pinned])

  const togglePin = () => {
    const next = !pinned
    setPinned(next)
    void api.popupPin(next)
  }

  const totalTabs =
    (state.local?.hits.length ?? 0) + state.runs.filter(run => run.result || run.error).length

  return (
    <div class="popup" ref={shellRef}>
      <div class="popup-head" ref={headRef} data-tauri-drag-region>
        <span class="popup-word" data-tauri-drag-region>
          {state.word || '沙拉划词'}
        </span>
        <button
          class={`icon-btn${pinned ? ' active' : ''}`}
          onClick={togglePin}
          title={pinned ? '取消置顶' : '置顶'}
        >
          <Icon name="pin" size={14} />
        </button>
        <button class="icon-btn" onClick={() => void api.popupHide()} title="关闭 (Esc)">
          <Icon name="close" size={14} />
        </button>
      </div>

      <div class="popup-body-area" ref={bodyRef}>
        <ResultView state={state} onEntry={word => lookup(word, 'manual')} />
      </div>

      <div class="popup-foot" ref={footRef}>
        <span>
          {SOURCE_LABEL[source] ?? '查询'} · {totalTabs} 个来源
        </span>
        <span>{state.elapsedMs ? `本地 ${state.elapsedMs} ms` : '查询中…'}</span>
      </div>
    </div>
  )
}
