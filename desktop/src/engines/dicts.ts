/**
 * 在线词典引擎：选择器移植自沙拉查词 src/components/dictionaries/*\/engine.ts。
 * 每个引擎只做「抓页面 → 提取核心字段 → 拼一段 HTML」，不依赖框架。
 */
import {
  absolutize,
  esc,
  innerHtml,
  parseHtml,
  renderDefs,
  renderExamples,
  renderTitle,
  section,
  text,
  textAll,
  type Pron
} from '../util'
import { HttpError, fetchJson, fetchPage, normalizeWord, request } from './http'
import type { Engine, EngineResult } from './index'

const NOISE =
  'script,style,noscript,iframe,form,svg,nav,footer,.ads,.ad,.share,.social,.promo,' +
  '.feedback-link,.dict-section__footer,.breadcrumb,.header-ad,.related,.recommend'

/** 整块抓取：去掉广告/脚本等噪声后原样渲染（用于结构复杂、字段零散的站点） */
function sweep(doc: Document, selector: string, host?: string): string {
  const el = doc.querySelector(selector)
  if (!el) return ''
  const clone = el.cloneNode(true) as Element
  clone.querySelectorAll(NOISE).forEach(node => node.remove())
  if (host) absolutize(clone, host)
  return clone.innerHTML
}

function stripToText(html: string): string {
  const tmp = document.createElement('div')
  tmp.innerHTML = html
  return (tmp.textContent || '').replace(/\s+/g, ' ').trim()
}

/* ---------------------------------- 英文 ---------------------------------- */

const YD_HOST = 'https://dict.youdao.com'

const youdao: Engine = {
  id: 'youdao',
  name: '有道',
  kind: 'dict',
  async search(word, ctx) {
    const url = YD_HOST + '/w/' + encodeURIComponent(normalizeWord(word))
    const doc = parseHtml(await fetchPage(url, YD_HOST + '/'))
    if (ctx.signal.aborted || doc.querySelector('#errorPage')) return null

    const title = text(doc, '.keyword') || text(doc, '.wordbook-js .word')
    const voice = doc.querySelector('.dictvoice[data-rel]')
    const audioUrl = voice
      ? YD_HOST + '/dictvoice?audio=' + encodeURIComponent(voice.getAttribute('data-rel') || '')
      : ''
    const prons: Pron[] = Array.from(doc.querySelectorAll('.baav .pronounce')).map((el, i) => ({
      label: '',
      value: (el.textContent || '').trim(),
      audio: i === 0 ? audioUrl : ''
    }))

    let defs = textAll(doc, '#phrsListTab .trans-container > ul > li').map(def => ({ def }))
    if (!defs.length) {
      defs = textAll(doc, '#phrsListTab .trans-container p').map(def => ({ def }))
    }

    const collins = innerHtml(doc, '#collinsResult', 'http://www.youdao.com')
    const sentences = innerHtml(doc, '#authority .ol', 'http://www.youdao.com')
    const translation = text(doc, '#fanyiToggle .trans-container')
    if (!title && !defs.length && !collins && !translation) return null

    return {
      title: title || '有道词典',
      html:
        renderTitle(title, prons) +
        (translation ? `<p class="res-title" style="font-size:16px">${esc(translation)}</p>` : '') +
        renderDefs(defs) +
        section('柯林斯', collins) +
        section('例句', sentences),
      audio: audioUrl ? { us: audioUrl } : undefined,
      sourceUrl: url
    }
  }
}

const BING_HOST = 'https://cn.bing.com'
const BING_SEARCH =
  BING_HOST + '/dict/clientsearch?mkt=zh-CN&setLang=zh&form=BDVEHC&ClientVer=BDDTV3.5.1.4320&q='
const BING_AUDIO_ATTRS = ['data-pronunciation', 'data-mp3link', 'audiomd5']

function bingAudio(node: Element | null): string {
  if (!node) return ''
  const candidates: Element[] = [
    node,
    ...Array.from(node.querySelectorAll('[data-pronunciation],[data-mp3link],[audiomd5],[onclick]'))
  ]
  for (const el of candidates) {
    for (const attr of BING_AUDIO_ATTRS) {
      const value = el.getAttribute(attr)
      if (value) return new URL(value, BING_HOST).href
    }
    const matched = (el.getAttribute('onclick') || '').match(
      /((?:https?:)?\/\/[^'")\s]+\.mp3(?:\?[^'")\s]*)?|\/[^'")\s]+\.mp3(?:\?[^'")\s]*)?)/
    )
    if (matched) return new URL(matched[1], BING_HOST).href
  }
  return ''
}

const bing: Engine = {
  id: 'bing',
  name: '必应',
  kind: 'dict',
  async search(word, ctx) {
    const doc = parseHtml(await fetchPage(BING_SEARCH + encodeURIComponent(normalizeWord(word))))
    if (ctx.signal.aborted) return null

    if (doc.querySelector('.client_def_hd_hd')) {
      const title = text(doc, '.client_def_hd_hd')
      const prons: Pron[] = Array.from(doc.querySelectorAll('.client_def_hd_pn_list')).map(el => ({
        label: text(el, '.client_def_hd_pn'),
        value: '',
        audio: bingAudio(el)
      }))
      const defs = Array.from(doc.querySelectorAll('.client_def_container .client_def_bar')).map(
        el => ({ pos: text(el, '.client_def_title_bar'), def: text(el, '.client_def_list') })
      )
      const infs = textAll(doc, '.client_word_change_word')
      const sentences = Array.from(doc.querySelectorAll('.client_sentence_list'))
        .slice(0, 4)
        .map(el => ({ en: text(el, '.client_sen_en'), zh: text(el, '.client_sen_cn') }))
        .filter(item => item.en || item.zh)
      if (!defs.length && !sentences.length && !prons.some(p => p.audio)) return null

      const audio: { us?: string; uk?: string } = {}
      prons.forEach(p => {
        if (!p.audio) return
        if (/us|美/i.test(p.label)) audio.us = p.audio
        if (/uk|英/i.test(p.label)) audio.uk = p.audio
      })

      return {
        title: title || '必应词典',
        html:
          renderTitle(title, prons) +
          renderDefs(defs) +
          (infs.length
            ? section('词形变化', `<p class="muted">${esc(infs.join(' · '))}</p>`)
            : '') +
          renderExamples(sentences),
        audio,
        sourceUrl: BING_SEARCH + encodeURIComponent(title || word)
      }
    }

    const machine = text(doc, '.client_sen_cn')
    if (machine) {
      return {
        title: '机器翻译',
        html:
          `<p class="res-title" style="font-size:17px">${esc(machine)}</p>` +
          (text(doc, '.client_sen_en')
            ? `<p class="muted">${esc(text(doc, '.client_sen_en'))}</p>`
            : '')
      }
    }

    const rows: string[] = []
    doc.querySelectorAll('.client_do_you_mean_list').forEach(list => {
      const item = text(list, '.client_do_you_mean_list_word')
      if (!item) return
      rows.push(
        `<li class="def"><span><a data-entry="${esc(item)}" href="#">${esc(item)}</a> ` +
          `${esc(text(list, '.client_do_you_mean_list_def'))}</span></li>`
      )
    })
    if (!rows.length) return null
    return { title: '你是不是想查', html: `<ul class="defs">${rows.join('')}</ul>` }
  }
}

const CAM_HOST = 'https://dictionary.cambridge.org'

const cambridge: Engine = {
  id: 'cambridge',
  name: '剑桥',
  kind: 'dict',
  async search(word, ctx) {
    const target = normalizeWord(word).split(/\s+/).join('-')
    const url = `${CAM_HOST}/dictionary/english-chinese-simplified/${encodeURIComponent(target)}`
    let html: string
    try {
      html = await fetchPage(url, CAM_HOST + '/')
    } catch (err) {
      if (err instanceof HttpError && err.status === 403) {
        throw new Error('剑桥词典触发了人机验证，请稍后再试或改用其它词典')
      }
      throw err
    }
    if (ctx.signal.aborted) return null
    const doc = parseHtml(html)

    const block = doc.querySelector('.entry-body__el')
    if (!block) return null

    const title = text(block, '.di-title') || text(doc, '.headword')
    const audioOf = (accent: string) => {
      const node = block.querySelector(`.dpron-i.${accent} source[type="audio/mpeg"]`)
      const src = node?.getAttribute('src') || ''
      return src ? new URL(src, CAM_HOST).href : ''
    }
    const uk = audioOf('uk')
    const us = audioOf('us')
    const prons: Pron[] = []
    if (uk) prons.push({ label: 'UK', value: '', audio: uk })
    if (us) prons.push({ label: 'US', value: '', audio: us })

    const defs: Array<{ pos?: string; def: string }> = []
    doc.querySelectorAll('.entry-body__el').forEach(el => {
      const pos = text(el, '.posgram')
      el.querySelectorAll('.def-block').forEach(defBlock => {
        const meaning = text(defBlock, '.def')
        const chinese = text(defBlock, '.trans')
        if (meaning || chinese) {
          defs.push({ pos: pos || undefined, def: [meaning, chinese].filter(Boolean).join(' ') })
        }
      })
    })

    const idiom = text(doc, '.idiom-block .idiom')
    if (!defs.length && !idiom) return null

    return {
      title: title || '剑桥词典',
      html:
        renderTitle(title, prons) +
        renderDefs(defs.slice(0, 12)) +
        (idiom ? section('习语', `<p class="muted">${esc(idiom)}</p>`) : ''),
      audio: { uk: uk || undefined, us: us || undefined },
      sourceUrl: url
    }
  }
}

const MW_HOST = 'https://www.merriam-webster.com'

const merriamwebster: Engine = {
  id: 'merriamwebster',
  name: '韦氏',
  kind: 'dict',
  async search(word, ctx) {
    const url = MW_HOST + '/dictionary/' + encodeURIComponent(normalizeWord(word))
    const doc = parseHtml(await fetchPage(url, MW_HOST + '/'))
    if (ctx.signal.aborted) return null

    const entry = doc.querySelector('#left-content div[id^="dictionary-entry"]')
    if (!entry) return null

    const title = text(entry, '.entry-header-content .hword')
    const pos = textAll(entry, 'h2.parts-of-speech a.important-blue-link').join(', ')
    const prons: Pron[] = Array.from(
      entry.querySelectorAll('span.prons-entries-list-inline .prons-entry-list-item')
    ).map(node => {
      const dir = node.getAttribute('data-dir') || ''
      const file = node.getAttribute('data-file') || ''
      const lang = (node.getAttribute('data-lang') || 'en/us').replace('/', '-')
      return {
        label: '',
        value: (node.querySelector('a') || node).textContent?.trim() || '',
        audio:
          dir && file
            ? `https://media.merriam-webster.com/audio/prons/${lang.replace('-', '/')}/mp3/${dir}/${file}.mp3`
            : ''
      }
    })
    const defs = textAll(entry, 'span.dtText')
      .slice(0, 12)
      .map(def => ({ pos: pos || undefined, def }))
    if (!defs.length) return null

    return {
      title: title || '韦氏词典',
      html:
        renderTitle(title, prons) +
        (text(entry, 'span.word-syllables-entry')
          ? `<p class="muted">${esc(text(entry, 'span.word-syllables-entry'))}</p>`
          : '') +
        renderDefs(defs) +
        renderExamples(
          textAll(entry, 'span.ex-sent.sents')
            .slice(0, 3)
            .map(en => ({ en }))
        ),
      audio: prons.find(p => p.audio)?.audio ? { us: prons.find(p => p.audio)!.audio } : undefined,
      sourceUrl: url
    }
  }
}

const COLLINS_HOST = 'https://www.collinsdictionary.com'

const collins: Engine = {
  id: 'collins',
  name: '柯林斯',
  kind: 'dict',
  async search(word, ctx) {
    const target = encodeURIComponent(normalizeWord(word).replace(/\s+/g, '-').toLowerCase())
    const url = `${COLLINS_HOST}/dictionary/english/${target}`
    const doc = parseHtml(await fetchPage(url, COLLINS_HOST + '/'))
    if (ctx.signal.aborted) return null

    const body = sweep(doc, '#main_content', COLLINS_HOST)
    if (!body) throw new Error('柯林斯页面结构变化，解析失败（可先配置代理再试）')
    const title = text(doc, '.h1_entry, h1.homograph, h1')
    return {
      title: title || '柯林斯',
      html: renderTitle(title) + `<div class="defs-rich">${body}</div>`,
      sourceUrl: url
    }
  }
}

const LONGMAN_HOST = 'https://www.ldoceonline.com'

const longman: Engine = {
  id: 'longman',
  name: '朗文',
  kind: 'dict',
  async search(word, ctx) {
    const url = LONGMAN_HOST + '/dictionary/' + encodeURIComponent(normalizeWord(word))
    const doc = parseHtml(await fetchPage(url, LONGMAN_HOST + '/'))
    if (ctx.signal.aborted) return null

    const entry = doc.querySelector('.dictentry')
    if (!entry) return null

    const head = entry.querySelector('.Head')
    const title = head ? text(head, '.HWD') : ''
    const prons: Pron[] = head
      ? [{ label: '', value: text(head, '.PronCodes'), audio: '' }]
      : []
    const senses = Array.from(entry.querySelectorAll('.Sense'))
      .slice(0, 10)
      .map(el => ({ pos: text(el, '.POS'), def: stripToText(innerHtml(el, '', LONGMAN_HOST)) || text(el) }))
      .filter(item => item.def)
    if (!senses.length) return null

    return {
      title: title || '朗文',
      html:
        renderTitle(title, prons) +
        renderDefs(senses) +
        section('搭配', innerHtml(entry, '.ColloBox', LONGMAN_HOST)) +
        section('语法', innerHtml(entry, '.GramBox', LONGMAN_HOST)),
      sourceUrl: url
    }
  }
}

const OALD_HOST = 'https://www.oxfordlearnersdictionaries.com'

const oaldict: Engine = {
  id: 'oaldict',
  name: '牛津',
  kind: 'dict',
  async search(word, ctx) {
    const url = OALD_HOST + '/search/english/direct/?q=' + encodeURIComponent(normalizeWord(word))
    const doc = parseHtml(await fetchPage(url, OALD_HOST + '/'))
    if (ctx.signal.aborted) return null

    const main = doc.querySelector('#entryContent')
    if (!main) return null

    const title = text(main, '.top-container .webtop .headword')
    const prons: Pron[] = Array.from(main.querySelectorAll('.top-container .phonetics > div')).map(
      el => {
        const sound = el.querySelector('.sound')
        const src = sound?.getAttribute('data-src-mp3') || ''
        return {
          label: text(el, '.phons_br') ? 'UK' : text(el, '.phons_n_am') ? 'US' : '',
          value: text(el, '.phon'),
          audio: src ? new URL(src, OALD_HOST).href : ''
        }
      }
    )
    const defs = Array.from(main.querySelectorAll('.entry > ol > .sense, .pv-g .sense'))
      .slice(0, 12)
      .map(el => ({
        pos: text(el, '.grammar, .pos'),
        def: stripToText(innerHtml(el, '.def', OALD_HOST)) || stripToText(innerHtml(el, '', OALD_HOST))
      }))
      .filter(item => item.def)
    if (!defs.length) return null

    return {
      title: title || '牛津',
      html: renderTitle(title, prons) + renderDefs(defs),
      audio: prons.find(p => p.audio)?.audio ? { uk: prons.find(p => p.audio)!.audio } : undefined,
      sourceUrl: url
    }
  }
}

const VOCAB_HOST = 'https://www.vocabulary.com'

const vocabulary: Engine = {
  id: 'vocabulary',
  name: 'Vocabulary',
  kind: 'dict',
  async search(word, ctx) {
    const url = VOCAB_HOST + '/dictionary/' + encodeURIComponent(normalizeWord(word))
    const doc = parseHtml(await fetchPage(url, VOCAB_HOST + '/'))
    if (ctx.signal.aborted) return null

    const short = text(doc, '.short')
    const long = text(doc, '.long')
    if (!short && !long) return null

    const defs = [{ def: short || long }]
    return {
      title: text(doc, 'h1') || normalizeWord(word),
      html:
        renderTitle(text(doc, 'h1') || normalizeWord(word)) +
        renderDefs(defs) +
        (long ? section('释义', `<p>${esc(long)}</p>`) : ''),
      sourceUrl: url
    }
  }
}

const ETY_HOST = 'https://www.etymonline.com'

const etymonline: Engine = {
  id: 'etymonline',
  name: '词源',
  kind: 'dict',
  async search(word, ctx) {
    const target = encodeURIComponent(normalizeWord(word))
    const url = ETY_HOST + '/word/' + target
    let doc = parseHtml(await fetchPage(url, ETY_HOST + '/'))
    if (!doc.querySelector('[class*="word__defination--"]')) {
      doc = parseHtml(await fetchPage(`${ETY_HOST}/search?q=${target}`, ETY_HOST + '/'))
    }
    if (ctx.signal.aborted) return null

    const item = doc.querySelector('[class*="word--"]')
    if (!item) return null
    const title = text(item, '[class*="word__name--"]')
    const def = innerHtml(item, '[class*="word__defination--"]', ETY_HOST)
    if (!def) return null

    return {
      title: title || '词源',
      html: renderTitle(title) + `<div class="defs-rich">${def}</div>`,
      sourceUrl: url
    }
  }
}

/* ------------------------------- 其它语种 ------------------------------- */

const HJ_HOST = 'https://dict.hujiang.com'

const hjdict: Engine = {
  id: 'hjdict',
  name: '沪江',
  kind: 'dict',
  async search(word, ctx) {
    const target = normalizeWord(word)
    const lang = /^[\x00-\x7f]+$/.test(target) ? 'en' : 'jp'
    const url = `${HJ_HOST}/${lang}/${encodeURIComponent(target)}`
    const doc = parseHtml(await fetchPage(url, HJ_HOST + '/'))
    if (ctx.signal.aborted || doc.querySelector('.word-notfound')) return null

    const panes = Array.from(doc.querySelectorAll('.word-details-pane')).slice(0, 4)
    if (!panes.length) return null

    const body = panes
      .map(pane => {
        const head = innerHtml(pane, '.word-details-pane-header', HJ_HOST)
        const content = innerHtml(pane, '.word-details-pane-content', HJ_HOST)
        return content ? `<div class="sec-title">${stripToText(head) || '释义'}</div><div class="defs-rich">${content}</div>` : ''
      })
      .join('')
    if (!body) return null

    const title = text(doc, '.word-details-multi .word-details-header h1, .word-details-header h1')
    return { title: title || target, html: renderTitle(title || target) + body, sourceUrl: url }
  }
}

const AHD_HOST = 'https://ahdictionary.com'

const ahdict: Engine = {
  id: 'ahdict',
  name: 'AHD',
  kind: 'dict',
  async search(word, ctx) {
    const url = AHD_HOST + '/word/search.html?q=' + encodeURIComponent(normalizeWord(word))
    const doc = parseHtml(await fetchPage(url, AHD_HOST + '/'))
    if (ctx.signal.aborted) return null

    const title = text(doc, '#results .rtseg a font') || text(doc, '.rtseg a')
    const defs = Array.from(doc.querySelectorAll('#results .pseg'))
      .slice(0, 8)
      .map(el => ({ def: stripToText(el.innerHTML) }))
      .filter(item => item.def)
    if (!defs.length) return null

    return {
      title: title || '美国传统词典',
      html:
        renderTitle(title) +
        renderDefs(defs) +
        section('用法', innerHtml(doc, '#results .usen', AHD_HOST)) +
        section('词源', innerHtml(doc, '#results .etyseg', AHD_HOST)),
      sourceUrl: url
    }
  }
}

const EUDIC_HOST = 'https://dict.eudic.net'

const eudic: Engine = {
  id: 'eudic',
  name: '欧路',
  kind: 'dict',
  async search(word, ctx) {
    const url = EUDIC_HOST + '/dicts/en/' + encodeURIComponent(normalizeWord(word))
    const doc = parseHtml(await fetchPage(url, EUDIC_HOST + '/'))
    if (ctx.signal.aborted) return null

    const title = text(doc, '.dict-word, .word-inline, h1')
    const body = sweep(doc, '.dict-body, .dict-area, #dict-body', EUDIC_HOST)
    if (!body) throw new Error('欧路页面结构变化，解析失败')
    return {
      title: title || '欧路词典',
      html: renderTitle(title) + `<div class="defs-rich">${body}</div>`,
      sourceUrl: url
    }
  }
}

interface NaverItem {
  entryName?: string
  meansCollector?: Array<{
    partOfSpeech?: string
    means?: Array<{ value?: string }>
  }>
}

const naver: Engine = {
  id: 'naver',
  name: 'Naver',
  kind: 'dict',
  async search(word, ctx) {
    const target = normalizeWord(word)
    const isJa = /[\u3040-\u30ff]/.test(target)
    const url = isJa
      ? `https://ja.dict.naver.com/api3/jako/search?query=${encodeURIComponent(target)}`
      : `https://zh.dict.naver.com/api3/zhko/search?query=${encodeURIComponent(target)}`
    const data = await fetchJson<{
      searchResultMap?: {
        searchResultListMap?: { WORD?: { items?: NaverItem[] } }
      }
    }>(url, 'https://dict.naver.com/')
    if (ctx.signal.aborted) return null

    const item = data.searchResultMap?.searchResultListMap?.WORD?.items?.[0]
    if (!item) return null
    const defs = (item.meansCollector || []).flatMap(collector =>
      (collector.means || [])
        .filter(mean => mean.value)
        .map(mean => ({ pos: collector.partOfSpeech, def: mean.value || '' }))
    )
    if (!defs.length) return null

    const title = item.entryName || target
    return { title, html: renderTitle(title) + renderDefs(defs.slice(0, 12)), sourceUrl: url }
  }
}

/* ---------------------------------- 日语 ---------------------------------- */

const WEBLIO_HOST = 'https://www.weblio.jp'

const weblio: Engine = {
  id: 'weblio',
  name: 'Weblio',
  kind: 'dict',
  async search(word, ctx) {
    const url = WEBLIO_HOST + '/content/' + encodeURIComponent(normalizeWord(word))
    const doc = parseHtml(await fetchPage(url, WEBLIO_HOST + '/'))
    if (ctx.signal.aborted) return null

    const cards = Array.from(doc.querySelectorAll('.section-card .basic-card')).slice(0, 4)
    const body = cards
      .map(card => {
        const head = text(card, '.pbarT h2')
        const content = innerHtml(card, '.NetDicBody, .pbarB, .basic-card-item', WEBLIO_HOST)
        return content ? `<div class="sec-title">${esc(head || '释义')}</div><div class="defs-rich">${content}</div>` : ''
      })
      .join('')
    if (!body) return null
    return { title: text(doc, 'h1') || 'Weblio', html: renderTitle(text(doc, 'h1') || 'Weblio') + body, sourceUrl: url }
  }
}

const EJJ_HOST = 'https://ejje.weblio.jp'

const weblioejje: Engine = {
  id: 'weblioejje',
  name: 'Weblio英和',
  kind: 'dict',
  async search(word, ctx) {
    const url = EJJ_HOST + '/content/' + encodeURIComponent(normalizeWord(word))
    const doc = parseHtml(await fetchPage(url, EJJ_HOST + '/'))
    if (ctx.signal.aborted) return null

    const entry = doc.querySelector('#summary, .summaryTbl, #main')
    if (!entry) return null
    const body = sweep(doc, '#summary', EJJ_HOST) || sweep(doc, '.summaryTbl', EJJ_HOST)
    if (!body) throw new Error('Weblio 页面结构变化，解析失败')
    const source = entry.querySelector('audio source, .summaryC audio source')
    const audio = source?.getAttribute('src') || ''
    const title = text(doc, '.summary-title h1') || text(doc, 'h1') || normalizeWord(word)
    return {
      title,
      html: renderTitle(title, audio ? [{ label: '', value: '', audio }] : []) + `<div class="defs-rich">${body}</div>`,
      sourceUrl: url
    }
  }
}

/* ------------------------------- 其它/俚语 ------------------------------- */

const SHANBAY_HOST = 'https://www.shanbay.com'

const shanbay: Engine = {
  id: 'shanbay',
  name: '扇贝',
  kind: 'dict',
  async search(word, ctx) {
    const target = normalizeWord(word)
    const url = SHANBAY_HOST + '/bdc/mobile/preview/word?word=' + encodeURIComponent(target)
    const doc = parseHtml(await fetchPage(url, SHANBAY_HOST + '/'))
    if (ctx.signal.aborted) return null

    const title = text(doc, '.word-spell')
    const body = innerHtml(doc, '.definition-cn', 'http://www.shanbay.com')
    if (!title || !body) return null

    const audio = {
      uk: `http://media.shanbay.com/audio/uk/${encodeURIComponent(title)}.mp3`,
      us: `http://media.shanbay.com/audio/us/${encodeURIComponent(title)}.mp3`
    }
    return {
      title,
      html:
        renderTitle(title, [
          { label: 'UK', value: '', audio: audio.uk },
          { label: 'US', value: '', audio: audio.us }
        ]) +
        (text(doc, '.word-announace') ? `<p class="muted">${esc(text(doc, '.word-announace'))}</p>` : '') +
        `<div class="defs-rich">${body}</div>`,
      audio,
      sourceUrl: url
    }
  }
}

const RENREN_HOST = 'https://www.91dict.com'

const renren: Engine = {
  id: 'renren',
  name: '人人词典',
  kind: 'dict',
  async search(word, ctx) {
    const target = normalizeWord(word)
    const url = RENREN_HOST + '/words?w=' + encodeURIComponent(target)
    const doc = parseHtml(await fetchPage(url, RENREN_HOST + '/'))
    if (ctx.signal.aborted) return null

    const slides = Array.from(doc.querySelectorAll('.tmInfo .slides > li')).slice(0, 3)
    if (!slides.length) return null

    const body = slides
      .map(li => {
        const en = text(li, '.mBottom')
        const chs = text(li, '.mFoot')
        const cover = li.querySelector('img')?.getAttribute('src') || ''
        const audio = li.querySelector('audio source')?.getAttribute('src') || ''
        if (!en && !chs) return ''
        return (
          `<div class="example">` +
          (cover ? `<img src="${esc(new URL(cover, RENREN_HOST).href)}" alt="" style="max-width:100%;border-radius:8px" />` : '') +
          `<div>${esc(en)}${audio ? ` <button type="button" class="speaker" data-audio="${esc(audio)}">▶</button>` : ''}</div>` +
          (chs ? `<div class="zh">${esc(chs)}</div>` : '') +
          `</div>`
        )
      })
      .join('')
    if (!body) return null

    const title = text(doc, '.tmInfo .mTop') || target
    return { title, html: renderTitle(title) + section('影视例句', body), sourceUrl: url }
  }
}

const baidu: Engine = {
  id: 'baidu',
  name: '百度',
  kind: 'dict',
  async search(word, ctx) {
    const target = normalizeWord(word)
    // fanyi.baidu.com/sug 是词典建议接口：无需签名，返回中英对照释义
    const body = await request({
      url: 'https://fanyi.baidu.com/sug',
      method: 'POST',
      referer: 'https://fanyi.baidu.com/',
      body: new URLSearchParams({ kw: target }).toString(),
      headers: { 'content-type': 'application/x-www-form-urlencoded' }
    })
    if (ctx.signal.aborted) return null
    const data = JSON.parse(body) as { data?: Array<{ k?: string; v?: string }> }
    const defs = (data.data || []).filter(item => item.v).map(item => ({ def: item.v || '' }))
    if (!defs.length) return null
    return { title: target, html: renderTitle(target) + renderDefs(defs.slice(0, 8)) }
  }
}

/* ---------------------------------- 维基 ---------------------------------- */

interface WikiSummary {
  title?: string
  extract?: string
  description?: string
  type?: string
  thumbnail?: { source?: string }
  content_urls?: { desktop?: { page?: string } }
}

const wikipedia: Engine = {
  id: 'wikipedia',
  name: '维基',
  kind: 'dict',
  async search(word, ctx) {
    const target = encodeURIComponent(normalizeWord(word))
    for (const lang of ['zh', 'en']) {
      try {
        const data = await fetchJson<WikiSummary>(
          `https://${lang}.wikipedia.org/api/rest_v1/page/summary/${target}`,
          `https://${lang}.wikipedia.org/`
        )
        if (ctx.signal.aborted) return null
        if (!data.extract || data.type === 'disambiguation') continue
        const paragraphs = data.extract
          .split(/\n+/)
          .filter(Boolean)
          .slice(0, 4)
          .map(para => `<p>${esc(para)}</p>`)
          .join('')
        const thumb = data.thumbnail?.source
          ? `<img src="${esc(data.thumbnail.source)}" alt="" style="float:right;max-width:96px;margin:0 0 8px 10px" />`
          : ''
        return {
          title: data.title || normalizeWord(word),
          html:
            renderTitle(data.title || normalizeWord(word)) +
            (data.description ? `<p class="muted">${esc(data.description)}</p>` : '') +
            thumb +
            section('摘要', paragraphs),
          sourceUrl: data.content_urls?.desktop?.page
        } satisfies EngineResult
      } catch (err) {
        if (err instanceof HttpError && (err.status === 404 || err.status === 400)) continue
        throw err
      }
    }
    return null
  }
}

interface UrbanItem {
  word: string
  definition: string
  example: string
  thumbs_up: number
  thumbs_down: number
  permalink: string
}

const urban: Engine = {
  id: 'urban',
  name: 'Urban',
  kind: 'dict',
  async search(word, ctx) {
    const data = await fetchJson<{ list?: UrbanItem[] }>(
      'https://api.urbandictionary.com/v0/define?term=' + encodeURIComponent(normalizeWord(word)),
      'https://www.urbandictionary.com/'
    )
    if (ctx.signal.aborted) return null
    const items = (data.list || []).slice(0, 3)
    if (!items.length) return null

    const body = items
      .map(item => {
        const clean = (input: string) => esc(input.replace(/[[\]]/g, ''))
        return (
          `<div class="example"><div>${clean(item.definition)}</div>` +
          (item.example ? `<div class="zh">${clean(item.example)}</div>` : '') +
          `<div class="muted" style="font-size:11px">👍 ${item.thumbs_up} · 👎 ${item.thumbs_down}</div></div>`
        )
      })
      .join('')

    return {
      title: items[0].word,
      html: renderTitle(items[0].word) + section('俚语释义', body),
      sourceUrl: items[0].permalink
    }
  }
}

export const DICTS: Engine[] = [
  youdao,
  bing,
  cambridge,
  merriamwebster,
  collins,
  longman,
  oaldict,
  vocabulary,
  etymonline,
  hjdict,
  ahdict,
  eudic,
  naver,
  weblio,
  weblioejje,
  shanbay,
  renren,
  baidu,
  wikipedia,
  urban
]
