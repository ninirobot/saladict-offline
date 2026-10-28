import { listen } from '@tauri-apps/api/event'
import { useCallback, useEffect, useRef, useState } from 'preact/hooks'

import { api } from '../api'
import { emptyLookup, startLookup, type LookupState } from '../lookup'
import { applyTheme } from '../util'
import type { Bootstrap, DictInfo } from '../types'
import { DictManager } from './DictManager'
import { HistoryList } from './HistoryList'
import { ResultView } from './ResultView'
import { SettingsPanel } from './SettingsPanel'
import { Icon } from './icons'

type TabKey = 'lookup' | 'dicts' | 'settings' | 'history'

const NAV: Array<{ key: TabKey; label: string; icon: string }> = [
  { key: 'lookup', label: '查词', icon: 'search' },
  { key: 'dicts', label: '词库', icon: 'book' },
  { key: 'settings', label: '设置', icon: 'slider' },
  { key: 'history', label: '历史', icon: 'clock' }
]

const TITLES: Record<TabKey, string> = {
  lookup: '查词',
  dicts: '词库',
  settings: '偏好设置',
  history: '查询历史'
}

export function MainWindow() {
  const [boot, setBoot] = useState<Bootstrap | null>(null)
  const [tab, setTab] = useState<TabKey>('lookup')
  const [state, setState] = useState<LookupState>(emptyLookup)
  const [input, setInput] = useState('')
  const [notice, setNotice] = useState('')
  const cancelRef = useRef<null | (() => void)>(null)

  const refresh = useCallback(async () => {
    const next = await api.bootstrap()
    setBoot(next)
    applyTheme(next.config.ui.theme)
    return next
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh])

  useEffect(() => {
    const offs: Array<() => void> = []
    void listen('open-settings', () => setTab('settings')).then(fn => offs.push(fn))
    void listen('history-changed', () => {
      void api.bootstrap().then(next => setBoot(next))
    }).then(fn => offs.push(fn))
    return () => offs.forEach(fn => fn())
  }, [])

  useEffect(() => () => cancelRef.current?.(), [])

  const query = (word: string) => {
    const text = word.trim()
    if (!text || !boot) return
    setInput(text)
    cancelRef.current?.()
    cancelRef.current = startLookup(text, {
      enabledEngines: boot.config.online.network ? boot.config.online.enabled : [],
      keys: boot.config.engineKeys ?? {},
      source: 'manual',
      onUpdate: patch => setState(prev => ({ ...prev, ...patch }))
    })
  }

  const updateConfig = (next: Bootstrap['config']) => {
    setBoot(prev => (prev ? { ...prev, config: next } : prev))
    applyTheme(next.ui.theme)
  }

  const updateDicts = (dicts: DictInfo[]) => {
    setBoot(prev => (prev ? { ...prev, dicts } : prev))
  }

  return (
    <div class="shell">
      <nav class="nav">
        <div class="brand">
          <span class="brand-dot">S</span>
          沙拉划词
        </div>
        {NAV.map(item => (
          <button
            key={item.key}
            class={`nav-item${tab === item.key ? ' active' : ''}`}
            onClick={() => {
              setTab(item.key)
              if (item.key === 'history') void refresh()
            }}
          >
            <Icon name={item.icon} size={15} />
            {item.label}
          </button>
        ))}
        <div class="nav-foot">
          v{boot?.version ?? '0.1.0'}
          <br />
          {boot?.paused ? '划词已暂停' : '划词监听中'}
          <br />
          本地词库 {boot?.dicts.length ?? 0} 本
        </div>
      </nav>

      <section class="content">
        <header class="content-head">
          <span class="content-title">{TITLES[tab]}</span>
        </header>
        <div class="content-body">
          {tab === 'lookup' && (
            <div>
              <div class="search-box">
                <Icon name="search" size={15} />
                <input
                  placeholder="输入单词或句子，回车查询（也可直接划词）"
                  value={input}
                  onInput={event => setInput((event.target as HTMLInputElement).value)}
                  onKeyDown={event => {
                    if (event.key === 'Enter') query(input)
                  }}
                />
                <button class="primary-btn" onClick={() => query(input)}>
                  查询
                </button>
              </div>

              <div style="margin-top:16px">
                {state.word ? (
                  <ResultView state={state} onEntry={query} />
                ) : (
                  <div class="empty">
                    <strong>划词即查</strong>
                    <span>
                      在浏览器、PDF 或 Word 里选中文字，浮窗会自动弹出释义；
                      <br />
                      也可以按 {boot?.config.selection.hotkeyCombo ?? 'Ctrl+Alt+D'} 读取当前选中内容。
                    </span>
                  </div>
                )}
              </div>
            </div>
          )}

          {tab === 'dicts' && (
            <DictManager dicts={boot?.dicts ?? []} onChange={updateDicts} />
          )}

          {tab === 'settings' && boot && (
            <div>
              {notice && <p class="err">{notice}</p>}
              <SettingsPanel config={boot.config} onChange={updateConfig} onNotice={setNotice} />
            </div>
          )}

          {tab === 'history' && (
            <HistoryList
              items={boot?.history ?? []}
              onSelect={word => {
                setTab('lookup')
                query(word)
              }}
              onRefresh={() => void refresh()}
            />
          )}
        </div>
      </section>
    </div>
  )
}
