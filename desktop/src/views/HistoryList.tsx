import { useMemo, useState } from 'preact/hooks'

import { api } from '../api'
import type { HistoryItem } from '../types'
import { Icon } from './icons'

interface Props {
  items: HistoryItem[]
  onSelect: (word: string) => void
  onRefresh: () => void
}

function dayLabel(at: number): string {
  const date = new Date(at)
  const today = new Date()
  const sameDay = date.toDateString() === today.toDateString()
  if (sameDay) return '今天'
  const yesterday = new Date(today.getTime() - 86400000)
  if (date.toDateString() === yesterday.toDateString()) return '昨天'
  return `${date.getMonth() + 1} 月 ${date.getDate()} 日`
}

function timeLabel(at: number): string {
  const date = new Date(at)
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`
}

export function HistoryList({ items, onSelect, onRefresh }: Props) {
  const [keyword, setKeyword] = useState('')

  const filtered = useMemo(() => {
    const key = keyword.trim().toLowerCase()
    if (!key) return items
    return items.filter(item => item.word.toLowerCase().includes(key))
  }, [items, keyword])

  const grouped = useMemo(() => {
    const groups = new Map<string, HistoryItem[]>()
    filtered.forEach(item => {
      const label = dayLabel(item.at)
      const list = groups.get(label) ?? []
      list.push(item)
      groups.set(label, list)
    })
    return Array.from(groups.entries())
  }, [filtered])

  return (
    <div>
      <div class="row" style="border-bottom:none;padding-top:0">
        <div class="search-box" style="flex:1">
          <Icon name="search" size={15} />
          <input
            placeholder="在历史里搜索"
            value={keyword}
            onInput={event => setKeyword((event.target as HTMLInputElement).value)}
          />
        </div>
        <button
          class="ghost-btn danger-btn"
          onClick={async () => {
            await api.historyClear()
            onRefresh()
          }}
        >
          清空
        </button>
      </div>

      {grouped.length === 0 ? (
        <div class="empty">
          <strong>还没有查询记录</strong>
          <span>划词查询过的词条会出现在这里，点击即可重新查询。</span>
        </div>
      ) : (
        grouped.map(([label, list]) => (
          <div key={label} style="margin-bottom:12px">
            <div class="sec-title">
              {label} · {list.length}
            </div>
            <div class="history-list">
              {list.map(item => (
                <div class="history-item" key={`${item.at}-${item.word}`}>
                  <button class="history-word" onClick={() => onSelect(item.word)}>
                    {item.word}
                  </button>
                  <span class="history-time">{timeLabel(item.at)}</span>
                  <button
                    class="history-del"
                    title="删除这条记录"
                    onClick={async () => {
                      await api.historyDelete(item.at)
                      onRefresh()
                    }}
                  >
                    <Icon name="close" size={12} />
                  </button>
                </div>
              ))}
            </div>
          </div>
        ))
      )}
    </div>
  )
}
