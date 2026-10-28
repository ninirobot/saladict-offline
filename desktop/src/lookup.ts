import { api } from './api'
import { runEngines, type EngineRun } from './engines'
import type { DictLookup } from './types'

export interface LookupState {
  word: string
  local: DictLookup | null
  runs: EngineRun[]
  elapsedMs: number
  pending: boolean
  error?: string
}

export const emptyLookup: LookupState = {
  word: '',
  local: null,
  runs: [],
  elapsedMs: 0,
  pending: false
}

interface StartOptions {
  enabledEngines: string[]
  keys: Record<string, string>
  source?: string
  onUpdate: (patch: Partial<LookupState>) => void
}

/**
 * 一次查询 = 本地词库（Rust） + 在线引擎（WebView 侧并发）。
 * 两边各自完成就各自回报，先到的先渲染。
 */
export function startLookup(word: string, options: StartOptions): () => void {
  const controller = new AbortController()
  const enabled = options.enabledEngines
  let outstanding = enabled.length ? 2 : 1

  options.onUpdate({
    ...emptyLookup,
    word,
    pending: true
  })

  const finishOne = () => {
    outstanding -= 1
    if (outstanding <= 0 && !controller.signal.aborted) {
      options.onUpdate({ pending: false })
    }
  }

  api
    .lookup(word, options.source ?? 'manual')
    .then(res => {
      if (controller.signal.aborted) return
      options.onUpdate({ local: res.local, elapsedMs: res.elapsedMs })
    })
    .catch(err => {
      if (controller.signal.aborted) return
      options.onUpdate({ error: String((err as Error)?.message || err) })
    })
    .finally(finishOne)

  if (enabled.length) {
    runEngines(
      word,
      enabled,
      options.keys,
      runs => {
        if (!controller.signal.aborted) options.onUpdate({ runs })
      },
      controller.signal
    )
      .then(runs => {
        if (!controller.signal.aborted) options.onUpdate({ runs })
      })
      .catch(() => undefined)
      .finally(finishOne)
  }

  return () => controller.abort()
}
