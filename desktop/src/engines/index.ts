import { api } from '../api'
import { DICTS } from './dicts'
import { MTS } from './mt'

export interface EngineResult {
  title: string
  /** 已组装好的 HTML 片段，渲染前还会过一遍 DOMPurify */
  html: string
  audio?: { us?: string; uk?: string }
  /** 原站词条页，便于核对 */
  sourceUrl?: string
}

/** 需要用户填写的凭据字段（只有机器翻译引擎会用到） */
export interface KeyField {
  key: string
  label: string
  placeholder?: string
}

export interface EngineCtx {
  signal: AbortSignal
  /** 取当前引擎的某个凭据，未填返回空串 */
  key: (field: string) => string
}

export interface Engine {
  id: string
  name: string
  kind: 'dict' | 'mt'
  fields?: KeyField[]
  /** 查不到返回 null；网络或解析异常直接抛错 */
  search(word: string, ctx: EngineCtx): Promise<EngineResult | null>
}

export interface EngineRun {
  id: string
  name: string
  result?: EngineResult
  error?: string
}

export const ENGINES: Engine[] = [...DICTS, ...MTS]

export const ENGINE_MAP = new Map(ENGINES.map(engine => [engine.id, engine]))

const TIMEOUT_MS = 9000

function withTimeout<T>(task: Promise<T>, signal: AbortSignal): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('请求超时')), TIMEOUT_MS)
    const abort = () => reject(new Error('已取消'))
    if (signal.aborted) abort()
    signal.addEventListener('abort', abort, { once: true })
    task.then(
      value => {
        clearTimeout(timer)
        signal.removeEventListener('abort', abort)
        resolve(value)
      },
      err => {
        clearTimeout(timer)
        signal.removeEventListener('abort', abort)
        reject(err)
      }
    )
  })
}

/**
 * 并发跑所有启用的引擎，谁先回来就先渲染谁。
 * keys 形如 { 'deepl.authKey': 'xxx' }，由设置面板写入。
 */
export async function runEngines(
  word: string,
  enabledIds: string[],
  keys: Record<string, string>,
  onProgress: (runs: EngineRun[]) => void,
  signal: AbortSignal
): Promise<EngineRun[]> {
  const ids = enabledIds.filter(id => ENGINE_MAP.has(id))
  const runs: EngineRun[] = ids.map(id => ({
    id,
    name: ENGINE_MAP.get(id)?.name ?? id
  }))
  const emit = () => onProgress(runs.map(run => ({ ...run })))
  emit()

  await Promise.all(
    runs.map(async run => {
      const engine = ENGINE_MAP.get(run.id)
      if (!engine) return
      const ctx: EngineCtx = {
        signal,
        key: field => (keys[`${run.id}.${field}`] || '').trim()
      }
      try {
        const result = await withTimeout(engine.search(word, ctx), signal)
        if (result) run.result = result
      } catch (err) {
        if (!signal.aborted) {
          run.error = (err as Error).message || String(err)
          void api.log(`engine ${run.id} failed: ${run.error}`, 'warn')
        }
      } finally {
        emit()
      }
    })
  )

  return runs.filter(run => run.result || run.error)
}
