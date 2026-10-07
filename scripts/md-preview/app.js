const file = decodeURIComponent(location.pathname)
const enc = file.split('/').map(encodeURIComponent).join('/')
const name = file.split('/').pop()

document.getElementById('name').textContent = name
document.getElementById('path').textContent = file
document.title = name + ' — md-preview'

const contentEl = document.getElementById('content')
const gutterEl = document.getElementById('gutter')
const statusEl = document.getElementById('status')
const sideList = document.getElementById('side-list')
const countEl = document.getElementById('count')
const toastEl = document.getElementById('toast')

let srcLines = []
let notes = []
let anchor = null // {l0, l1} of the last gutter click
let pending = null // {l0, l1} currently being commented on
let cursorLine = null
let rendered = false

function toast(msg) {
  toastEl.textContent = msg
  toastEl.classList.add('show')
  clearTimeout(toast.t)
  toast.t = setTimeout(() => toastEl.classList.remove('show'), 1600)
}

function setStatus(cls, text) {
  statusEl.className = 'pill ' + cls
  statusEl.textContent = text
}

// ─── storage ──────────────────────────────────────────

async function loadNotes() {
  try {
    const res = await fetch('/__notes' + enc, { cache: 'no-store' })
    notes = (await res.json()).notes || []
  } catch {
    notes = []
  }
}

let saving = Promise.resolve()

function save() {
  countEl.textContent = notes.length
  const body = JSON.stringify({ notes })
  saving = saving
    .then(() => fetch('/__notes' + enc, { method: 'PUT', headers: { 'content-type': 'application/json' }, body }))
    .then((res) => {
      if (!res.ok) throw new Error(res.status)
    })
    .catch(() => toast('annotations not saved — server unreachable'))
}

// ─── markdown ─────────────────────────────────────────

function slugify(s) {
  return s
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s_-]/gu, '')
    .replace(/\s/g, '-')
}

const md = window
  .markdownit({ html: true, linkify: true, breaks: false })
  .use(window.markdownitTaskLists, { label: true })
  .use(window.markdownitFootnote)
  .use(window.markdownItAnchor, { slugify, tabIndex: false })

md.core.ruler.push('source_lines', (state) => {
  for (const tok of state.tokens) {
    if (!tok.map) continue
    if (!tok.type.endsWith('_open') && !['fence', 'hr', 'code_block'].includes(tok.type)) continue
    tok.attrSet('data-l0', String(tok.map[0] + 1))
    tok.attrSet('data-l1', String(tok.map[1]))
  }
})

// markdown-it's own fence renderer puts the token's attributes on the
// inner <code>, leaving the <pre> unanchored.
md.renderer.rules.fence = (tokens, idx) => {
  const tok = tokens[idx]
  const lang = (tok.info || '').trim().split(/\s+/)[0]
  const l0 = tok.attrGet('data-l0')
  const anchor = l0 ? ` data-l0="${l0}" data-l1="${tok.attrGet('data-l1')}"` : ''
  if (lang === 'mermaid') {
    return `<pre class="mermaid"${anchor}>${md.utils.escapeHtml(tok.content)}</pre>\n`
  }
  let body = ''
  if (lang && hljs.getLanguage(lang)) {
    try {
      body = hljs.highlight(tok.content, { language: lang }).value
    } catch {}
  }
  if (!body) body = md.utils.escapeHtml(tok.content)
  const cls = lang ? ` class="language-${md.utils.escapeHtml(lang)}"` : ''
  return `<pre${anchor}><code${cls}>${body}</code></pre>\n`
}

const css = (v) => getComputedStyle(document.documentElement).getPropertyValue(v).trim()

mermaid.initialize({
  startOnLoad: false,
  securityLevel: 'strict',
  theme: 'base',
  fontFamily: css('--sans'),
  themeVariables: {
    darkMode: true,
    background: css('--bg'),
    primaryColor: css('--surface-hi'),
    primaryTextColor: css('--fg'),
    primaryBorderColor: css('--accent'),
    secondaryColor: css('--surface'),
    tertiaryColor: css('--bg'),
    lineColor: css('--fg-muted'),
    textColor: css('--fg'),
    mainBkg: css('--surface-hi'),
    nodeBorder: css('--accent'),
    clusterBkg: css('--surface'),
    clusterBorder: css('--border'),
    titleColor: css('--fg-strong'),
    edgeLabelBackground: css('--surface'),
    noteBkgColor: css('--surface-hi'),
    noteTextColor: css('--fg'),
    noteBorderColor: css('--attention'),
  },
})

// Blanked rather than cut, so markdown-it's line map still matches the file.
function splitFrontmatter(src) {
  const lines = src.split('\n')
  if (lines[0].trim() !== '---') return { body: src, meta: null }
  const end = lines.findIndex((l, i) => i > 0 && (l.trim() === '---' || l.trim() === '...'))
  if (end === -1) return { body: src, meta: null }
  const yaml = lines.slice(1, end).join('\n')
  const body = lines.map((l, i) => (i <= end ? '' : l)).join('\n')
  return { body, meta: { yaml, l1: end + 1 } }
}

function frontmatterEl(meta) {
  const el = document.createElement('details')
  el.className = 'frontmatter'
  el.dataset.l0 = '1'
  el.dataset.l1 = String(meta.l1)
  const summary = document.createElement('summary')
  summary.textContent = 'frontmatter'
  el.append(summary)

  let data
  try {
    data = jsyaml.load(meta.yaml)
  } catch {}

  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    const pre = document.createElement('pre')
    pre.textContent = meta.yaml
    el.append(pre)
    return el
  }

  const table = document.createElement('table')
  for (const [k, v] of Object.entries(data)) {
    const tr = document.createElement('tr')
    const th = document.createElement('th')
    th.textContent = k
    const td = document.createElement('td')
    if (v !== null && typeof v === 'object') {
      const pre = document.createElement('pre')
      pre.textContent = jsyaml.dump(v).trimEnd()
      td.append(pre)
    } else {
      td.textContent = String(v)
    }
    tr.append(th, td)
    table.append(tr)
  }
  el.append(table)
  return el
}

async function render(src) {
  const y = window.scrollY
  const open = contentEl.querySelector(':scope > details.frontmatter')?.open
  srcLines = src.split('\n')
  reanchor()
  const { body, meta } = splitFrontmatter(src)
  contentEl.innerHTML = DOMPurify.sanitize(md.render(body))
  if (meta) {
    const fm = frontmatterEl(meta)
    fm.open = !!open
    contentEl.prepend(fm)
  }
  const diagrams = contentEl.querySelectorAll('pre.mermaid')
  if (diagrams.length) {
    try {
      await mermaid.run({ nodes: diagrams, suppressErrors: true })
    } catch {}
  }
  paint()
  window.scrollTo({ top: y, behavior: 'instant' })
  if (!rendered) {
    rendered = true
    if (location.hash) document.getElementById(decodeURIComponent(location.hash.slice(1)))?.scrollIntoView()
    else if (cursorLine) follow(cursorLine)
  }
}

// ─── cursor sync ──────────────────────────────────────

function blockAt(line) {
  let best = null
  for (const el of contentEl.querySelectorAll('[data-l0]')) {
    const l0 = +el.dataset.l0
    const l1 = +el.dataset.l1
    if (l0 > line || l1 < line) continue
    if (!best || l1 - l0 <= +best.dataset.l1 - +best.dataset.l0) best = el
  }
  return best || hostFor(line)
}

function follow(line) {
  cursorLine = line
  if (!rendered || anchor || pending) return
  const el = blockAt(line)
  if (!el) return
  const r = el.getBoundingClientRect()
  const h = window.innerHeight
  if (r.top >= h * 0.15 && r.top <= h * 0.75) return
  el.scrollIntoView({ block: 'center', behavior: 'smooth' })
}

// ─── anchoring ────────────────────────────────────────

function findLine(text, near) {
  if (!text) return -1
  let best = -1
  for (let i = 0; i < srcLines.length; i++) {
    if (srcLines[i].trim() !== text) continue
    if (best === -1 || Math.abs(i - near) < Math.abs(best - near)) best = i
  }
  return best
}

function reanchor() {
  let moved = false
  for (const n of notes) {
    const was = `${n.l0}-${n.l1}-${n.orphan}`
    const cur = (srcLines[n.l0 - 1] || '').trim()
    if (cur === n.snip0) {
      n.orphan = false
    } else {
      const i = findLine(n.snip0, n.l0 - 1)
      if (i !== -1) {
        const span = n.l1 - n.l0
        n.l0 = i + 1
        const j = findLine(n.snip1, i + span)
        n.l1 = j !== -1 && j + 1 >= n.l0 ? j + 1 : n.l0 + span
        n.orphan = false
      } else {
        n.orphan = true
      }
    }
    if (`${n.l0}-${n.l1}-${n.orphan}` !== was) moved = true
  }
  if (moved) save()
}

function snippetOf(l) {
  return (srcLines[l - 1] || '').trim()
}

// ─── painting ─────────────────────────────────────────

function blocks() {
  return [...contentEl.querySelectorAll(':scope > [data-l0]')]
}

function covered(r) {
  const all = [...contentEl.querySelectorAll('[data-l0]')].filter((e) => +e.dataset.l0 >= r.l0 && +e.dataset.l1 <= r.l1)
  return all.filter((e) => !all.some((o) => o !== e && e.contains(o)))
}

function hostFor(l1) {
  const bs = blocks()
  let hit = null
  for (const b of bs) {
    if (+b.dataset.l0 <= l1) hit = b
    else break
  }
  return hit || bs[bs.length - 1] || null
}

function paintSelection() {
  contentEl.querySelectorAll('.sel').forEach((e) => e.classList.remove('sel'))
  if (pending) covered(pending).forEach((e) => e.classList.add('sel'))
}

function paint() {
  contentEl.querySelectorAll('.card, .composer').forEach((el) => el.remove())
  contentEl.querySelectorAll('.noted').forEach((e) => e.classList.remove('noted'))

  for (const n of notes) {
    covered(n).forEach((e) => e.classList.add('noted'))
    const host = hostFor(n.l1)
    if (host) host.after(cardFor(n))
  }

  paintSelection()
  if (pending) {
    const host = hostFor(pending.l1)
    if (host) host.after(composer())
    const ta = document.querySelector('.composer textarea')
    if (ta) ta.focus()
  }

  paintSide()
  countEl.textContent = notes.length
}

function label(n) {
  return n.l0 === n.l1 ? 'L' + n.l0 : 'L' + n.l0 + '-L' + n.l1
}

function cardFor(n) {
  const el = document.createElement('div')
  el.className = 'card' + (n.orphan ? ' orphan' : '')
  const meta = document.createElement('div')
  meta.className = 'meta'
  const lines = document.createElement('span')
  lines.className = 'lines'
  lines.textContent = label(n) + (n.orphan ? ' · orphaned' : '')
  const del = document.createElement('button')
  del.textContent = 'delete'
  del.onclick = () => {
    notes = notes.filter((x) => x.id !== n.id)
    save()
    paint()
  }
  meta.append(lines, del)
  const body = document.createElement('div')
  body.className = 'body'
  body.textContent = n.text
  el.append(meta, body)
  return el
}

function composer() {
  const el = document.createElement('div')
  el.className = 'composer'
  const meta = document.createElement('div')
  meta.className = 'meta'
  meta.textContent = label(pending)
  const ta = document.createElement('textarea')
  ta.placeholder = 'Comment…'
  const row = document.createElement('div')
  row.className = 'row'
  const ok = document.createElement('button')
  ok.className = 'pill accent'
  ok.textContent = 'save'
  const no = document.createElement('button')
  no.className = 'pill'
  no.textContent = 'cancel'
  const hint = document.createElement('span')
  hint.className = 'hint'
  hint.textContent = '⌘↵ save · esc cancel'
  row.append(ok, no, hint)

  const commit = () => {
    const text = ta.value.trim()
    if (text) {
      notes.push({
        id: Date.now() + '-' + Math.random().toString(36).slice(2, 7),
        l0: pending.l0,
        l1: pending.l1,
        snip0: snippetOf(pending.l0),
        snip1: snippetOf(pending.l1),
        text,
        orphan: false,
      })
      notes.sort((a, b) => a.l0 - b.l0)
      save()
    }
    pending = null
    anchor = null
    paint()
  }
  const abort = () => {
    pending = null
    anchor = null
    paint()
  }
  ok.onclick = commit
  no.onclick = abort
  ta.onkeydown = (e) => {
    if (e.key === 'Escape') abort()
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) commit()
  }
  el.append(meta, ta, row)
  return el
}

function paintSide() {
  sideList.innerHTML = ''
  if (!notes.length) {
    sideList.innerHTML =
      '<div class="empty">Hover a line and click the <b>+</b> in the left margin. Drag down the margin to cover several lines.</div>'
    return
  }
  for (const n of notes) {
    const el = document.createElement('div')
    el.className = 'item' + (n.orphan ? ' orphan' : '')
    const lines = document.createElement('div')
    lines.className = 'lines'
    lines.textContent = label(n) + (n.orphan ? ' · orphaned' : '')
    const body = document.createElement('div')
    body.className = 'body'
    body.textContent = n.text
    el.append(lines, body)
    el.onclick = () => {
      const host = blocks().find((b) => +b.dataset.l0 >= n.l0)
      if (host) host.scrollIntoView({ block: 'center' })
    }
    sideList.append(el)
  }
}

// ─── gutter interaction ───────────────────────────────

const docEl = document.getElementById('doc')
let hovered = null

function rangeOf(el) {
  return { l0: +el.dataset.l0, l1: +el.dataset.l1 }
}

function showGutter(el) {
  hovered = el
  const a = el.getBoundingClientRect()
  const b = docEl.getBoundingClientRect()
  gutterEl.style.top = a.top - b.top + 'px'
  gutterEl.classList.add('shown')
}

contentEl.addEventListener('mousemove', (e) => {
  if (anchor) return
  const el = e.target.closest('[data-l0]')
  if (el && el !== hovered) showGutter(el)
})

docEl.addEventListener('mouseleave', () => {
  if (anchor) return
  hovered = null
  gutterEl.classList.remove('shown')
})

gutterEl.addEventListener('mousedown', (e) => {
  if (!hovered) return
  e.preventDefault()
  anchor = rangeOf(hovered)
  pending = { ...anchor }
  document.body.classList.add('dragging')
  paintSelection()
})

document.addEventListener('mousemove', (e) => {
  if (!anchor) return
  const x = contentEl.getBoundingClientRect().left + 24
  const el = document.elementFromPoint(x, e.clientY)?.closest('[data-l0]')
  if (!el) return
  const r = rangeOf(el)
  pending = { l0: Math.min(anchor.l0, r.l0), l1: Math.max(anchor.l1, r.l1) }
  paintSelection()
})

document.addEventListener('mouseup', () => {
  if (!anchor) return
  anchor = null
  document.body.classList.remove('dragging')
  paint()
})

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && pending) {
    pending = null
    anchor = null
    paint()
  }
})

// ─── export ───────────────────────────────────────────

document.getElementById('copy-all').onclick = async () => {
  if (!notes.length) return toast('nothing to copy')
  const body = notes
    .slice()
    .sort((a, b) => a.l0 - b.l0)
    .map((n) => label(n) + (n.orphan ? ' (orphaned)' : '') + ': ' + n.text.replace(/\n/g, ' '))
    .join('\n')
  const out = file + '\n\n' + body + '\n'
  try {
    await navigator.clipboard.writeText(out)
    toast(notes.length + ' annotation' + (notes.length > 1 ? 's' : '') + ' copied')
  } catch {
    toast('clipboard blocked — see console')
    console.log(out)
  }
}

document.getElementById('toggle-side').onclick = () => document.body.classList.toggle('with-side')

// ─── live reload ──────────────────────────────────────

async function pull() {
  const res = await fetch('/__raw' + enc, { cache: 'no-store' })
  if (!res.ok) throw new Error('gone')
  await render(await res.text())
}

let es = null

function connect() {
  if (es) es.close()
  es = new EventSource('/__events' + enc)
  es.onopen = () => {
    setStatus('live', 'live')
    pull().catch(() => setStatus('dead', 'file gone'))
  }
  es.addEventListener('change', (ev) => {
    if (JSON.parse(ev.data).gone) return setStatus('dead', 'file gone')
    setStatus('live', 'live')
    pull().catch(() => setStatus('dead', 'file gone'))
  })
  es.addEventListener('cursor', (ev) => follow(JSON.parse(ev.data).line))
  es.onerror = () => setStatus('stale', 'reconnecting…')
}

// Chrome suspends background tabs, dropping the SSE stream with no error the
// page ever sees.
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState !== 'visible') return
  if (!es || es.readyState === EventSource.CLOSED) connect()
  else pull().catch(() => setStatus('dead', 'file gone'))
})

loadNotes()
  .then(pull)
  .then(connect)
  .catch(() => setStatus('dead', 'file gone'))
