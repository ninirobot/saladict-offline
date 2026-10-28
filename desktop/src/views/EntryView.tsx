import DOMPurify from 'dompurify'
import { useEffect, useMemo, useRef } from 'preact/hooks'

import { api } from '../api'
import { dataUrl, playBase64, playAudio } from '../util'

const FORBID_TAGS = ['style', 'link', 'iframe', 'object', 'embed', 'form', 'input', 'textarea']
const ADD_ATTR = [
  'data-entry',
  'data-sound',
  'data-audio',
  'data-mdx-src',
  'data-mdx-href',
  'target',
  'colspan',
  'rowspan'
]

export function sanitize(html: string): string {
  return DOMPurify.sanitize(html, {
    FORBID_TAGS,
    ADD_ATTR,
    ALLOW_DATA_ATTR: true
  }) as string
}

interface Props {
  html: string
  /** 词条所属词典 id：用于从 mdd 取图片与音频 */
  dictId?: string
  onEntry?: (word: string) => void
  class?: string
}

export function EntryView({ html, dictId, onEntry, class: className }: Props) {
  const ref = useRef<HTMLDivElement>(null)
  const clean = useMemo(() => sanitize(html), [html])

  // 词条图片走 IPC 取回，避免自定义协议在不同运行环境下的差异
  useEffect(() => {
    const root = ref.current
    if (!root || !dictId) return
    let cancelled = false
    const nodes = Array.from(root.querySelectorAll<HTMLImageElement>('img[data-mdx-src]'))

    nodes.forEach(async img => {
      const path = img.getAttribute('data-mdx-src')
      img.removeAttribute('data-mdx-src')
      if (!path) return
      const base64 = await api.readAsset(dictId, path).catch(() => null)
      if (cancelled) return
      if (base64) {
        img.src = dataUrl(path, base64)
      } else {
        img.style.display = 'none'
      }
    })

    return () => {
      cancelled = true
    }
  }, [clean, dictId])

  useEffect(() => {
    const root = ref.current
    if (!root) return

    const onClick = async (event: MouseEvent) => {
      const target = event.target as HTMLElement | null
      if (!target) return

      const soundNode = target.closest('[data-sound]')
      if (soundNode) {
        event.preventDefault()
        const path = soundNode.getAttribute('data-sound')
        if (path && dictId) {
          const base64 = await api.readAsset(dictId, path).catch(() => null)
          if (base64) playBase64(base64, path)
        }
        return
      }

      const audioNode = target.closest('[data-audio]')
      if (audioNode) {
        event.preventDefault()
        playAudio(audioNode.getAttribute('data-audio') || undefined)
        return
      }

      const entryNode = target.closest('[data-entry]')
      if (entryNode) {
        event.preventDefault()
        const word = entryNode.getAttribute('data-entry')
        if (word && onEntry) onEntry(decodeURIComponent(word))
        return
      }

      const link = target.closest('a[href]')
      if (link) {
        const href = link.getAttribute('href') || ''
        event.preventDefault()
        if (/^https?:/i.test(href)) {
          void api.openUrl(href)
        }
      }
    }

    root.addEventListener('click', onClick)
    return () => root.removeEventListener('click', onClick)
  }, [dictId, onEntry])

  return (
    <div
      ref={ref}
      class={`entry${className ? ' ' + className : ''}`}
      dangerouslySetInnerHTML={{ __html: clean }}
    />
  )
}
