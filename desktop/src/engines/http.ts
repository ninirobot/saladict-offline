import { api } from '../api'

export class HttpError extends Error {
  status: number
  constructor(status: number) {
    super(
      status === 403
        ? '目标站点拒绝了请求（可能需要人机验证，或在设置里配置代理）'
        : `请求失败 HTTP ${status}`
    )
    this.status = status
    this.name = 'HttpError'
  }
}

export interface Req {
  url: string
  referer?: string
  method?: 'GET' | 'POST'
  body?: string
  headers?: Record<string, string>
}

/** 所有网络请求都走 Rust 侧：没有 CORS 限制，可自定义 UA / Referer / 请求头 */
export async function request(req: Req): Promise<string> {
  const res = await api.fetch(req)
  if (res.status >= 400) throw new HttpError(res.status)
  return res.body
}

export function fetchPage(url: string, referer?: string): Promise<string> {
  return request({ url, referer })
}

export async function fetchJson<T>(url: string, referer?: string): Promise<T> {
  return JSON.parse(await request({ url, referer })) as T
}

export async function postJson<T>(
  url: string,
  payload: unknown,
  headers: Record<string, string> = {}
): Promise<T> {
  return JSON.parse(
    await request({
      url,
      method: 'POST',
      body: JSON.stringify(payload),
      headers: { 'content-type': 'application/json; charset=UTF-8', ...headers }
    })
  ) as T
}

export function normalizeWord(text: string): string {
  return text.replace(/\s+/g, ' ').trim()
}

/** 中文/日文/韩文任一字即视为需要译成英文 */
export const CJK = /[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uac00-\ud7af]/

export function targetLang(text: string): string {
  return CJK.test(text) ? 'en' : 'zh-CN'
}
