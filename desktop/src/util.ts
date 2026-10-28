/** 解析、转义、结果排版、mdd 媒体与主题：所有引擎和视图共用的最小工具集 */

/* ---------- HTML / DOM ---------- */

export function parseHtml(html: string): Document {
  return new DOMParser().parseFromString(html, 'text/html')
}

/** 取第一个匹配节点的文本，做 trim + 空白折叠 */
export function text(root: ParentNode | null, selector?: string): string {
  if (!root) return ''
  const el = selector ? root.querySelector(selector) : (root as Element)
  if (!el) return ''
  return collapse(el.textContent || '')
}

export function textAll(root: ParentNode | null, selector: string): string[] {
  if (!root) return []
  return Array.from(root.querySelectorAll(selector))
    .map(el => collapse(el.textContent || ''))
    .filter(Boolean)
}

/** 取 innerHTML（把相对链接绝对化，便于点开原站） */
export function innerHtml(root: ParentNode | null, selector: string, host?: string): string {
  if (!root) return ''
  const el = selector ? root.querySelector(selector) : (root as Element)
  if (!el) return ''
  if (!host) return el.innerHTML
  const clone = el.cloneNode(true) as Element
  absolutize(clone, host)
  return clone.innerHTML
}

/** 把元素内的相对链接/图片地址补全为绝对地址 */
export function absolutize(root: Element, host: string) {
  root.querySelectorAll('a[href]').forEach(a => {
    a.setAttribute('href', absUrl(host, a.getAttribute('href') || ''))
  })
  root.querySelectorAll('img[src]').forEach(img => {
    img.setAttribute('src', absUrl(host, img.getAttribute('src') || ''))
  })
}

export function absUrl(host: string, url: string): string {
  const raw = url.trim()
  if (!raw) return raw
  if (/^(https?:|data:|mailto:|#)/i.test(raw)) return raw
  if (raw.startsWith('//')) return 'https:' + raw
  return host.replace(/\/+$/, '') + '/' + raw.replace(/^\/+/, '')
}

export function collapse(input: string): string {
  return input.replace(/\s+/g, ' ').trim()
}

export function esc(input: string): string {
  return input
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

/* ---------- 统一结果排版 ---------- */

const SPEAKER_SVG =
  '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" ' +
  'stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
  '<path d="M11 5 6 9H3v6h3l5 4V5z"/><path d="M16 9a4 4 0 0 1 0 6"/></svg>'

export function speakerButton(audio?: string): string {
  if (!audio) return ''
  return `<button type="button" class="speaker" data-audio="${esc(audio)}" title="发音">${SPEAKER_SVG}</button>`
}

export interface Pron {
  label: string
  value: string
  audio?: string
}

export function renderTitle(title: string, prons: Pron[] = []): string {
  const parts: string[] = [`<h1 class="res-title">${esc(title)}</h1>`]
  if (prons.length) {
    const items = prons
      .filter(p => p.value || p.audio)
      .map(
        p =>
          `<span class="res-pron">${p.label ? `<em>${esc(p.label)}</em>` : ''}` +
          `${p.value ? `<span>${esc(p.value)}</span>` : ''}${speakerButton(p.audio)}</span>`
      )
      .join('')
    if (items) parts.push(`<div class="res-prons">${items}</div>`)
  }
  return parts.join('')
}

export interface Def {
  pos?: string
  def: string
  /** 允许带少量行内标签时传 true */
  html?: boolean
}

export function renderDefs(defs: Def[]): string {
  if (!defs.length) return ''
  const items = defs
    .map(
      d =>
        `<li class="def">${d.pos ? `<span class="pos">${esc(d.pos)}</span>` : ''}` +
        `<span>${d.html ? d.def : esc(d.def)}</span></li>`
    )
    .join('')
  return `<ul class="defs">${items}</ul>`
}

export function renderExamples(list: Array<{ en: string; zh?: string }>, title = '例句'): string {
  if (!list.length) return ''
  const items = list
    .map(
      e =>
        `<div class="example"><div>${e.en}</div>` +
        `${e.zh ? `<div class="zh">${e.zh}</div>` : ''}</div>`
    )
    .join('')
  return section(title, items)
}

export function section(title: string, html: string): string {
  if (!html) return ''
  return `<div class="sec-title">${esc(title)}</div>${html}`
}

/* ---------- mdd 媒体 ---------- */

const MIME: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  gif: 'image/gif',
  webp: 'image/webp',
  bmp: 'image/bmp',
  svg: 'image/svg+xml',
  mp3: 'audio/mpeg',
  wav: 'audio/wav',
  ogg: 'audio/ogg',
  spx: 'audio/ogg'
}

export function mimeOf(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  return MIME[ext] ?? 'application/octet-stream'
}

export function dataUrl(path: string, base64: string): string {
  return `data:${mimeOf(path)};base64,${base64}`
}

let current: HTMLAudioElement | null = null

export function playAudio(url?: string) {
  if (!url) return
  try {
    current?.pause()
    const audio = new Audio(url)
    current = audio
    void audio.play().catch(() => undefined)
  } catch {
    /* 播放失败静默 */
  }
}

export function playBase64(base64: string, path: string) {
  try {
    const binary = atob(base64)
    const bytes = new Uint8Array(binary.length)
    for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i)
    playAudio(URL.createObjectURL(new Blob([bytes], { type: mimeOf(path) })))
  } catch {
    /* ignore */
  }
}

/* ---------- 主题 ---------- */

export type Theme = 'auto' | 'light' | 'dark'

export function applyTheme(theme: Theme | string | undefined) {
  const root = document.documentElement
  if (!theme || theme === 'auto') {
    delete root.dataset.theme
  } else {
    root.dataset.theme = theme
  }
}
