/**
 * 机器翻译引擎。需要密钥的引擎在 `fields` 里声明字段，
 * 设置面板会自动渲染对应输入框，值存在 config.engineKeys。
 */
import { esc, renderTitle, section } from '../util'
import { CJK, fetchJson, normalizeWord, postJson, request, targetLang } from './http'
import type { Engine, EngineResult } from './index'

function result(translated: string, source: string, note?: string): EngineResult {
  return {
    title: '翻译',
    html:
      renderTitle(translated) +
      section('原文', `<p class="muted">${esc(source)}</p>`) +
      (note ? `<p class="muted" style="font-size:11px">${esc(note)}</p>` : '')
  }
}

/* --------------------------------- 免密钥 --------------------------------- */

const google: Engine = {
  id: 'google',
  name: '谷歌翻译',
  kind: 'mt',
  async search(word, ctx) {
    const source = normalizeWord(word)
    if (!source) return null
    const target = targetLang(source)
    const body = await request({
      url: `https://translate.googleapis.com/translate_a/single?client=gtx&dt=t&sl=auto&tl=${target}&q=${encodeURIComponent(source)}`,
      referer: 'https://translate.google.com/'
    })
    if (ctx.signal.aborted) return null
    const data = JSON.parse(body) as [Array<[string]>, unknown, string]
    const translated = (data[0] || []).map(seg => seg[0]).filter(Boolean).join('')
    if (!translated) return null
    return result(translated, source, `检测语言：${data[2] || 'auto'} → ${target}`)
  }
}

/* -------------------------------- 需要密钥 -------------------------------- */

const deepl: Engine = {
  id: 'deepl',
  name: 'DeepL',
  kind: 'mt',
  fields: [{ key: 'authKey', label: 'Auth Key', placeholder: 'xxxxxxxx-xxxx-...:fx' }],
  async search(word, ctx) {
    const source = normalizeWord(word)
    const key = ctx.key('authKey')
    if (!source) return null
    if (!key) throw new Error('请先在设置里填写 DeepL Auth Key')

    const target = CJK.test(source) ? 'EN' : 'ZH'
    const body = `text=${encodeURIComponent(source)}&target_lang=${target}`
    const headers = {
      Authorization: `DeepL-Auth-Key ${key}`,
      'content-type': 'application/x-www-form-urlencoded'
    }
    const hosts = ['https://api-free.deepl.com', 'https://api.deepl.com']
    for (const host of hosts) {
      try {
        const data = JSON.parse(
          await request({ url: `${host}/v2/translate`, method: 'POST', body, headers })
        ) as { translations?: Array<{ text?: string }> }
        const translated = data.translations?.[0]?.text
        if (translated) return result(translated, source, `→ ${target}`)
      } catch {
        // free 端点失败时再试 pro
      }
    }
    return null
  }
}

const deeplx: Engine = {
  id: 'deeplx',
  name: 'DeepLX',
  kind: 'mt',
  fields: [
    { key: 'endpoint', label: '服务地址', placeholder: 'http://127.0.0.1:1188' },
    { key: 'token', label: 'Token（可选）' }
  ],
  async search(word, ctx) {
    const source = normalizeWord(word)
    const endpoint = ctx.key('endpoint') || 'http://127.0.0.1:1188'
    if (!source) return null
    const target = CJK.test(source) ? 'EN' : 'ZH'
    const token = ctx.key('token')
    const data = await postJson<{ data?: string }>(
      `${endpoint.replace(/\/+$/, '')}/translate`,
      { text: source, source_lang: 'auto', target_lang: target },
      token ? { Authorization: `Bearer ${token}` } : {}
    )
    if (!data?.data) return null
    return result(data.data, source, `→ ${target}`)
  }
}

async function sha256(input: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(input))
  return Array.from(new Uint8Array(digest))
    .map(byte => byte.toString(16).padStart(2, '0'))
    .join('')
}

const youdaotrans: Engine = {
  id: 'youdaotrans',
  name: '有道翻译',
  kind: 'mt',
  fields: [
    { key: 'appKey', label: '应用 ID' },
    { key: 'appSecret', label: '应用密钥' }
  ],
  async search(word, ctx) {
    const source = normalizeWord(word)
    const appKey = ctx.key('appKey')
    const appSecret = ctx.key('appSecret')
    if (!source) return null
    if (!appKey || !appSecret) throw new Error('请先在设置里填写有道翻译的应用 ID 与密钥')

    const salt = String(Date.now())
    const curtime = String(Math.floor(Date.now() / 1000))
    const trimmed =
      source.length <= 20 ? source : `${source.slice(0, 10)}${source.length}${source.slice(-10)}`
    const sign = await sha256(appKey + trimmed + salt + curtime + appSecret)
    const form = new URLSearchParams({
      q: source,
      from: 'auto',
      to: CJK.test(source) ? 'en' : 'zh-CHS',
      appKey,
      salt,
      sign,
      signType: 'v3',
      curtime
    })
    const data = JSON.parse(
      await request({
        url: 'https://openapi.youdao.com/api',
        method: 'POST',
        body: form.toString(),
        headers: { 'content-type': 'application/x-www-form-urlencoded' }
      })
    ) as {
      translation?: string[]
      basic?: { explains?: string[] }
      errorCode?: string
    }
    const translated = data.translation?.[0]
    if (!translated) throw new Error(`有道返回错误码 ${data.errorCode ?? '未知'}`)
    const explains = (data.basic?.explains || []).map(item => `<p class="muted">${esc(item)}</p>`).join('')
    return {
      title: '翻译',
      html:
        renderTitle(translated) +
        explains +
        section('原文', `<p class="muted">${esc(source)}</p>`)
    }
  }
}

const caiyun: Engine = {
  id: 'caiyun',
  name: '彩云小译',
  kind: 'mt',
  fields: [{ key: 'token', label: 'Token' }],
  async search(word, ctx) {
    const source = normalizeWord(word)
    const token = ctx.key('token')
    if (!source) return null
    if (!token) throw new Error('请先在设置里填写彩云小译 Token')

    const data = await postJson<{ target?: string[] }>(
      'https://api.interpreter.caiyunai.com/v1/translator',
      {
        source: [source],
        trans_type: CJK.test(source) ? 'auto2en' : 'auto2zh',
        request_id: 'saladict',
        detect: true
      },
      { 'x-authorization': `token ${token}` }
    )
    const translated = data.target?.[0]
    if (!translated) return null
    return result(translated, source)
  }
}

const niutrans: Engine = {
  id: 'niutrans',
  name: '小牛翻译',
  kind: 'mt',
  fields: [{ key: 'apikey', label: 'API Key' }],
  async search(word, ctx) {
    const source = normalizeWord(word)
    const apikey = ctx.key('apikey')
    if (!source) return null
    if (!apikey) throw new Error('请先在设置里填写小牛翻译 API Key')

    const params = new URLSearchParams({
      from: 'auto',
      to: CJK.test(source) ? 'en' : 'zh',
      apikey,
      src_text: source
    })
    const data = await fetchJson<{ tgt_text?: string }>(
      'https://api.niutrans.com/NiuTransServer/translation?' + params.toString(),
      'https://niutrans.com/'
    )
    if (!data.tgt_text) return null
    return result(data.tgt_text, source)
  }
}

export const MTS: Engine[] = [google, deepl, deeplx, youdaotrans, caiyun, niutrans]
