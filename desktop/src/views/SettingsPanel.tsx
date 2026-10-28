import { useEffect, useState } from 'preact/hooks'

import { api } from '../api'
import { ENGINES } from '../engines'
import type { AppConfig } from '../types'

interface Props {
  config: AppConfig
  onChange: (config: AppConfig) => void
  onNotice: (message: string) => void
}

const THEMES: Array<{ value: AppConfig['ui']['theme']; label: string }> = [
  { value: 'auto', label: '跟随系统' },
  { value: 'light', label: '浅色' },
  { value: 'dark', label: '深色' }
]

function keyName(event: KeyboardEvent): string {
  if (/^F\d{1,2}$/.test(event.key)) return event.key.toUpperCase()
  if (event.key === ' ') return 'Space'
  if (event.key.length === 1 && /[a-z0-9]/i.test(event.key)) return event.key.toUpperCase()
  const named: Record<string, string> = {
    Enter: 'Enter',
    Tab: 'Tab',
    Backspace: 'Backspace',
    Delete: 'Delete',
    Insert: 'Insert',
    ArrowUp: 'Up',
    ArrowDown: 'Down',
    ArrowLeft: 'Left',
    ArrowRight: 'Right',
    '`': '`'
  }
  return named[event.key] ?? ''
}

export function SettingsPanel({ config, onChange, onNotice }: Props) {
  const [recording, setRecording] = useState(false)
  const [local, setLocal] = useState(config)

  useEffect(() => {
    setLocal(config)
  }, [config])

  const save = async (next: AppConfig) => {
    setLocal(next)
    try {
      const saved = await api.configSet(next)
      onChange(saved)
      onNotice('')
    } catch (err) {
      onNotice(String((err as Error)?.message || err))
      setLocal(config)
    }
  }

  const patchSelection = (patch: Partial<AppConfig['selection']>) =>
    void save({ ...local, selection: { ...local.selection, ...patch } })
  const patchUi = (patch: Partial<AppConfig['ui']>) =>
    void save({ ...local, ui: { ...local.ui, ...patch } })
  const patchOnline = (patch: Partial<AppConfig['online']>) =>
    void save({ ...local, online: { ...local.online, ...patch } })
  const setKey = (field: string, value: string) =>
    void save({ ...local, engineKeys: { ...local.engineKeys, [field]: value } })

  useEffect(() => {
    if (!recording) return
    const onKey = (event: KeyboardEvent) => {
      event.preventDefault()
      event.stopPropagation()
      if (event.key === 'Escape') {
        setRecording(false)
        return
      }
      const parts: string[] = []
      if (event.ctrlKey) parts.push('Ctrl')
      if (event.altKey) parts.push('Alt')
      if (event.shiftKey) parts.push('Shift')
      if (event.metaKey) parts.push('Win')
      const key = keyName(event)
      if (!key) return
      if (parts.length === 0) {
        onNotice('快捷键至少要包含一个修饰键，例如 Ctrl+Alt+D')
        return
      }
      setRecording(false)
      patchSelection({ hotkeyCombo: [...parts, key].join('+') })
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  })

  const toggleEngine = (id: string) => {
    const enabled = local.online.enabled.includes(id)
      ? local.online.enabled.filter(item => item !== id)
      : [...local.online.enabled, id]
    patchOnline({ enabled })
  }

  return (
    <div class="card">
      <div class="row">
        <div class="row-label">
          <div class="name">鼠标划词自动查询</div>
          <div class="desc">在任意程序里拖动选中文字，松开鼠标即弹出查词浮窗。</div>
        </div>
        <button
          class={`switch${local.selection.drag ? ' on' : ''}`}
          onClick={() => patchSelection({ drag: !local.selection.drag })}
        />
      </div>

      <div class="row">
        <div class="row-label">
          <div class="name">快捷键取词</div>
          <div class="desc">按快捷键读取当前焦点应用里的选中文字，不监听鼠标。</div>
        </div>
        <button
          class={`hotkey-input${recording ? ' recording' : ''}`}
          onClick={() => setRecording(true)}
          title="点击后按下组合键"
        >
          {recording ? '请按下组合键…' : local.selection.hotkeyCombo}
        </button>
        <button
          class={`switch${local.selection.hotkey ? ' on' : ''}`}
          onClick={() => patchSelection({ hotkey: !local.selection.hotkey })}
        />
      </div>

      <div class="row">
        <div class="row-label">
          <div class="name">取词长度限制</div>
          <div class="desc">太短的（如单个标点）和太长的段落都不查询，避免误触。</div>
        </div>
        <input
          class="hotkey-input"
          style="width:64px;text-align:center"
          type="number"
          min={1}
          max={40}
          value={local.selection.minChars}
          onInput={event =>
            patchSelection({ minChars: Number((event.target as HTMLInputElement).value) || 1 })
          }
        />
        <span class="muted">—</span>
        <input
          class="hotkey-input"
          style="width:72px;text-align:center"
          type="number"
          min={8}
          max={400}
          value={local.selection.maxChars}
          onInput={event =>
            patchSelection({ maxChars: Number((event.target as HTMLInputElement).value) || 80 })
          }
        />
      </div>

      <div class="row">
        <div class="row-label">
          <div class="name">划词后自动置顶</div>
          <div class="desc">浮窗不会因点击别处而消失，方便对照原文。</div>
        </div>
        <button
          class={`switch${local.selection.pinOnOpen ? ' on' : ''}`}
          onClick={() => patchSelection({ pinOnOpen: !local.selection.pinOnOpen })}
        />
      </div>

      <div class="row">
        <div class="row-label">
          <div class="name">外观主题</div>
          <div class="desc">深色模式跟随系统时，切换系统主题会立即生效。</div>
        </div>
        <div class="result-tabs" style="margin:0">
          {THEMES.map(theme => (
            <button
              key={theme.value}
              class={`tab${local.ui.theme === theme.value ? ' active' : ''}`}
              onClick={() => patchUi({ theme: theme.value })}
            >
              {theme.label}
            </button>
          ))}
        </div>
      </div>

      <div class="row">
        <div class="row-label">
          <div class="name">浮窗宽度</div>
          <div class="desc">300–640px，改动在下次划词时生效。</div>
        </div>
        <input
          class="hotkey-input"
          style="width:80px"
          type="number"
          min={300}
          max={640}
          step={4}
          value={local.ui.popupWidth}
          onInput={event =>
            patchUi({
              popupWidth: Math.min(
                640,
                Math.max(300, Number((event.target as HTMLInputElement).value) || 388)
              )
            })
          }
        />
      </div>

      <div class="row">
        <div class="row-label">
          <div class="name">在线查词</div>
          <div class="desc">
            由后台直接请求，无需浏览器跨域许可。关掉后只查本地词库，完全离线。
          </div>
        </div>
        <button
          class={`switch${local.online.network ? ' on' : ''}`}
          onClick={() => patchOnline({ network: !local.online.network })}
        />
      </div>

      <div class="row">
        <div class="row-label">
          <div class="name">网络代理</div>
          <div class="desc">
            留空为直连。韦氏/柯林斯/词源返回 403，或维基/Urban/谷歌翻译超时时，填一个代理即可。
          </div>
        </div>
        <input
          class="key-input"
          style="max-width:240px"
          placeholder="http://127.0.0.1:7890"
          value={local.online.proxy}
          onInput={event => patchOnline({ proxy: (event.target as HTMLInputElement).value })}
        />
      </div>

      <div class="row" style="align-items:flex-start">
        <div class="row-label">
          <div class="name">启用的在线词典</div>
          <div class="desc">可随时增减；新增引擎只需在 registry 里注册一次。</div>
          <div class="chips">
            {ENGINES.map(engine => {
              const on = local.online.enabled.includes(engine.id)
              return (
                <button
                  key={engine.id}
                  class="chip"
                  style={
                    on
                      ? 'color:var(--primary-deep);border-color:color-mix(in srgb, var(--primary) 45%, transparent);background:var(--primary-soft)'
                      : undefined
                  }
                  onClick={() => toggleEngine(engine.id)}
                >
                  {engine.name}
                  {engine.kind === 'mt' ? ' (翻译)' : ''}
                </button>
              )
            })}
          </div>

          {ENGINES.filter(
            engine => engine.fields && local.online.enabled.includes(engine.id)
          ).map(engine => (
            <div class="key-fields" key={engine.id}>
              <div class="desc">{engine.name} 凭据</div>
              {engine.fields?.map(field => (
                <input
                  class="key-input"
                  type={/secret|key|token|auth/i.test(field.key) ? 'password' : 'text'}
                  placeholder={field.placeholder || field.label}
                  value={local.engineKeys[`${engine.id}.${field.key}`] || ''}
                  onInput={event =>
                    setKey(`${engine.id}.${field.key}`, (event.target as HTMLInputElement).value)
                  }
                />
              ))}
            </div>
          ))}
        </div>
      </div>

      <div class="row">
        <div class="row-label">
          <div class="name">开机自动启动</div>
          <div class="desc">写入当前用户的启动项，随登录静默运行在托盘。</div>
        </div>
        <button
          class={`switch${local.autostart ? ' on' : ''}`}
          onClick={async () => {
            const enabled = !local.autostart
            try {
              await api.autostartSet(enabled)
              setLocal({ ...local, autostart: enabled })
              onChange({ ...local, autostart: enabled })
            } catch (err) {
              onNotice(String((err as Error)?.message || err))
            }
          }}
        />
      </div>

      <div class="row">
        <div class="row-label">
          <div class="name">历史记录上限</div>
          <div class="desc">超出后自动丢弃最旧的记录；只存词条，不存上下文。</div>
        </div>
        <input
          class="hotkey-input"
          style="width:80px"
          type="number"
          min={50}
          max={5000}
          step={50}
          value={local.historyLimit}
          onInput={event =>
            void save({
              ...local,
              historyLimit: Number((event.target as HTMLInputElement).value) || 500
            })
          }
        />
      </div>
    </div>
  )
}
