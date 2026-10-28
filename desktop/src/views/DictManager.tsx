import { useState } from 'preact/hooks'

import { api } from '../api'
import type { DictInfo } from '../types'
import { Icon } from './icons'

interface Props {
  dicts: DictInfo[]
  onChange: (dicts: DictInfo[]) => void
}

function formatSize(bytes: number): string {
  if (!bytes) return '—'
  const mb = bytes / 1024 / 1024
  return mb >= 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(1)} MB`
}

export function DictManager({ dicts, onChange }: Props) {
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const run = async (task: () => Promise<DictInfo[]>, okMessage?: string) => {
    setBusy(true)
    setMessage('')
    try {
      const next = await task()
      onChange(next)
      if (okMessage) setMessage(okMessage)
    } catch (err) {
      setMessage(String((err as Error)?.message || err))
    } finally {
      setBusy(false)
    }
  }

  const add = async () => {
    const path = await api.pickMdx()
    if (!path) return
    await run(() => api.dictAdd(path), '已添加词典，首次查询时才会建立索引')
  }

  return (
    <div>
      <div class="row" style="border-bottom:none;padding-top:0">
        <div class="row-label">
          <div class="name">本地词库</div>
          <div class="desc">
            支持 .mdx；同名 .mdd 会自动关联，用于词条里的图片与原生发音。首次查询时才建立索引，不会拖慢启动。
          </div>
        </div>
        <button class="primary-btn" onClick={add} disabled={busy}>
          {busy ? '处理中…' : '添加 mdx'}
        </button>
      </div>

      {message && <p class="muted">{message}</p>}

      {dicts.length === 0 ? (
        <div class="empty">
          <strong>还没有词库</strong>
          <span>添加一本 mdx 词典后，划词时就会先查本地词典，离线也能用。</span>
        </div>
      ) : (
        <div class="dict-list">
          {dicts.map((dict, index) => (
            <div class="dict-item" key={dict.id}>
              <button
                class="icon-btn"
                title="上移"
                disabled={index === 0}
                onClick={() =>
                  void run(() => api.dictMove(dict.id, Math.max(0, index - 1)))
                }
              >
                <Icon name="book" size={14} />
              </button>
              <div class="dict-main">
                <div class="dict-name">
                  {dict.name}
                  {dict.mdd ? <span class="tag" style="margin-left:6px">含 MDD</span> : null}
                  {dict.error ? (
                    <span class="tag" style="margin-left:6px;color:var(--danger)">解析异常</span>
                  ) : null}
                </div>
                <div class="dict-meta">
                  {dict.entryCount ? `${dict.entryCount.toLocaleString()} 词条 · ` : ''}
                  {formatSize(dict.size)} · {dict.path}
                </div>
                {dict.error && <div class="err">{dict.error}</div>}
              </div>
              <button
                class={`switch${dict.enabled ? ' on' : ''}`}
                title={dict.enabled ? '停用' : '启用'}
                onClick={() => void run(() => api.dictToggle(dict.id, !dict.enabled))}
              />
              <button
                class="icon-btn"
                title="重新打开词典"
                onClick={() => void run(() => api.dictRefresh(dict.id))}
              >
                <Icon name="refresh" size={14} />
              </button>
              <button
                class="icon-btn"
                title="移除"
                onClick={() => void run(() => api.dictRemove(dict.id))}
              >
                <Icon name="trash" size={14} />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
