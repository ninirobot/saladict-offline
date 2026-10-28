import { useEffect, useMemo, useState } from 'preact/hooks'

import type { LookupState } from '../lookup'
import { EntryView } from './EntryView'

interface TabItem {
  id: string
  name: string
  dictId?: string
  html?: string
  error?: string
}

interface Props {
  state: LookupState
  onEntry: (word: string) => void
}

export function ResultView({ state, onEntry }: Props) {
  const tabs = useMemo<TabItem[]>(() => {
    const items: TabItem[] = []
    state.local?.hits.forEach(hit => {
      items.push({ id: `mdx:${hit.id}`, name: hit.name, dictId: hit.id, html: hit.html })
    })
    state.local?.errors.forEach(err => {
      items.push({ id: `mdxerr:${err.id}`, name: err.name, error: err.message })
    })
    state.runs.forEach(run => {
      if (run.result) {
        items.push({
          id: `eng:${run.id}`,
          name: run.name,
          html: run.result.html
        })
      } else if (run.error) {
        items.push({ id: `eng:${run.id}`, name: run.name, error: run.error })
      }
    })
    return items
  }, [state.local, state.runs])

  const [active, setActive] = useState('')

  useEffect(() => {
    if (!tabs.length) {
      setActive('')
      return
    }
    if (!tabs.some(tab => tab.id === active)) {
      setActive(tabs[0].id)
    }
  }, [tabs, active])

  const current = tabs.find(tab => tab.id === active)

  if (!state.word) {
    return <div class="empty">选中或输入要查的词</div>
  }

  if (!tabs.length) {
    if (state.pending) {
      return (
        <div class="loading-row">
          <span class="spinner" />
          正在查询「{state.word}」…
        </div>
      )
    }
    const suggestions = state.local?.suggestions ?? []
    return (
      <div>
        <p class="muted">
          {state.error ? <span class="err">{state.error}</span> : `没有找到「${state.word}」的词条`}
        </p>
        {suggestions.length > 0 && (
          <>
            <div class="sec-title">相近词条</div>
            <div class="chips">
              {suggestions.map(word => (
                <button key={word} class="chip" onClick={() => onEntry(word)}>
                  {word}
                </button>
              ))}
            </div>
          </>
        )}
      </div>
    )
  }

  return (
    <div class="result-pane">
      <div class="result-tabs">
        {tabs.map(tab => (
          <button
            key={tab.id}
            class={`tab${tab.id === active ? ' active' : ''}`}
            onClick={() => setActive(tab.id)}
            title={tab.error ? '该词典返回了错误' : tab.name}
          >
            {tab.name}
            {tab.error ? ' ⚠' : ''}
          </button>
        ))}
        {state.pending && <span class="spinner" style="margin:4px 0 0 4px" />}
      </div>

      {current?.error ? (
        <p class="err">{current.error}</p>
      ) : current?.html ? (
        <EntryView html={current.html} dictId={current.dictId} onEntry={onEntry} />
      ) : null}
    </div>
  )
}
