import { existsSync, mkdirSync, readFileSync, realpathSync, renameSync, statSync, unlinkSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join, resolve, sep } from 'node:path'
import { HOST, LABEL, MARKDOWN, PORT } from './config.js'

const HERE = dirname(realpathSync(import.meta.path))
const NOTES = join(homedir(), '.local/share/md-preview/notes')
const POLL_MS = 250
const PING_MS = 30_000
const MAX_BODY = 1024 * 1024

const LIBS = {
  'markdown-it.js': 'markdown-it/dist/markdown-it.min.js',
  'markdown-it-anchor.js': 'markdown-it-anchor/dist/markdownItAnchor.umd.js',
  'markdown-it-footnote.js': 'markdown-it-footnote/dist/markdown-it-footnote.min.js',
  'markdown-it-task-lists.js': 'markdown-it-task-lists/dist/markdown-it-task-lists.min.js',
  // highlight.js ships no browser bundle on npm; @highlightjs/cdn-assets is the one that does.
  'highlight.js': '@highlightjs/cdn-assets/highlight.min.js',
  'mermaid.js': 'mermaid/dist/mermaid.min.js',
  'purify.js': 'dompurify/dist/purify.min.js',
  'js-yaml.js': 'js-yaml/dist/js-yaml.min.js',
}

const HOSTS = new Set([`${HOST}:${PORT}`, `localhost:${PORT}`])

const CSP = [
  "default-src 'none'",
  "script-src 'self'",
  "style-src 'self' 'unsafe-inline'",
  'img-src * data: blob:',
  "font-src 'self' data:",
  "connect-src 'self'",
  "base-uri 'none'",
  "form-action 'none'",
  "frame-ancestors 'none'",
].join('; ')

const enc = new TextEncoder()

// ─── paths ──────────────────────────────────────────────

function fromUrl(pathname, prefix = '') {
  try {
    return resolve('/', decodeURIComponent(pathname.slice(prefix.length)))
  } catch {
    return null
  }
}

function real(p) {
  try {
    return realpathSync(p)
  } catch {
    return null
  }
}

function mtime(p) {
  try {
    return statSync(p).mtimeMs
  } catch {
    return 0
  }
}

function isFile(p) {
  try {
    return statSync(p).isFile()
  } catch {
    return false
  }
}

function within(child, root) {
  return child === root || child.startsWith(root + sep)
}

const roots = new Map()

function rootOf(doc) {
  const dir = dirname(doc)
  if (!roots.has(dir)) {
    const git = Bun.spawnSync(['git', '-C', dir, 'rev-parse', '--show-toplevel'])
    const top = git.success ? git.stdout.toString().trim() : ''
    roots.set(dir, real(top || dir) || dir)
  }
  return roots.get(dir)
}

// ─── live state per file ────────────────────────────────

const watched = new Map()

function entry(file) {
  let w = watched.get(file)
  if (!w) {
    w = { clients: new Set(), mtime: mtime(file), cursor: null }
    watched.set(file, w)
  }
  return w
}

function frame(event, data) {
  return `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`
}

function send(w, chunk) {
  for (const c of w.clients) {
    try {
      c.enqueue(enc.encode(chunk))
    } catch {
      w.clients.delete(c)
    }
  }
}

function events(file) {
  const w = entry(file)
  let self
  const stream = new ReadableStream({
    start(controller) {
      self = controller
      w.clients.add(self)
      self.enqueue(enc.encode(': connected\n\n'))
      if (w.cursor) self.enqueue(enc.encode(frame('cursor', { line: w.cursor })))
    },
    cancel() {
      w.clients.delete(self)
    },
  })
  return new Response(stream, {
    headers: { 'content-type': 'text/event-stream', 'cache-control': 'no-store', connection: 'keep-alive' },
  })
}

// ─── notes ──────────────────────────────────────────────

function notesPath(file) {
  return join(NOTES, new Bun.CryptoHasher('sha1').update(file).digest('hex') + '.json')
}

function getNotes(file) {
  try {
    return Response.json({ notes: JSON.parse(readFileSync(notesPath(file), 'utf8')).notes || [] })
  } catch {
    return Response.json({ notes: [] })
  }
}

async function putNotes(req, file) {
  const body = await jsonBody(req)
  if (!body || !Array.isArray(body.notes)) return new Response('bad request', { status: 400 })
  const dest = notesPath(file)
  if (!body.notes.length) {
    if (existsSync(dest)) unlinkSync(dest)
    return new Response(null, { status: 204 })
  }
  mkdirSync(NOTES, { recursive: true })
  const tmp = `${dest}.${process.pid}.tmp`
  writeFileSync(tmp, JSON.stringify({ path: file, notes: body.notes }, null, 2) + '\n')
  renameSync(tmp, dest)
  return new Response(null, { status: 204 })
}

// A JSON content type cannot be sent cross-origin without a CORS preflight,
// which this server never answers.
async function jsonBody(req) {
  if (!(req.headers.get('content-type') || '').startsWith('application/json')) return null
  if (Number(req.headers.get('content-length') || 0) > MAX_BODY) return null
  try {
    return await req.json()
  } catch {
    return null
  }
}

// ─── routes ─────────────────────────────────────────────

function shell() {
  return new Response(readFileSync(join(HERE, 'index.html')), {
    headers: {
      'content-type': 'text/html; charset=utf-8',
      'content-security-policy': CSP,
      'cache-control': 'no-store',
    },
  })
}

function lib(name) {
  const rel = LIBS[name]
  if (!rel) return new Response('not found', { status: 404 })
  const p = join(HERE, 'node_modules', rel)
  if (!existsSync(p)) return new Response(`missing ${rel}: run bun install in ${HERE}`, { status: 500 })
  return new Response(Bun.file(p), { headers: { 'content-type': 'text/javascript; charset=utf-8' } })
}

function raw(file) {
  if (!file || !MARKDOWN.test(file) || !isFile(file)) return new Response('gone', { status: 404 })
  return new Response(readFileSync(file), {
    headers: { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' },
  })
}

function referrerDoc(req) {
  try {
    const u = new URL(req.headers.get('referer'))
    if (!HOSTS.has(u.host)) return null
    const doc = fromUrl(u.pathname)
    return doc && MARKDOWN.test(doc) ? real(doc) : null
  } catch {
    return null
  }
}

function asset(req, path) {
  const doc = referrerDoc(req)
  if (!doc) return new Response('forbidden', { status: 403 })
  const target = real(path)
  if (!target || !isFile(target)) return new Response('not found', { status: 404 })
  if (!within(target, rootOf(doc))) return new Response('forbidden', { status: 403 })
  return new Response(Bun.file(target))
}

async function handle(req) {
  if (!HOSTS.has(req.headers.get('host'))) return new Response('forbidden', { status: 403 })

  const url = new URL(req.url)
  const p = url.pathname

  if (p === '/__meta') return Response.json({ app: 'md-preview', pid: process.pid })
  if (p === '/__app.js') {
    return new Response(Bun.file(join(HERE, 'app.js')), {
      headers: { 'content-type': 'text/javascript; charset=utf-8', 'cache-control': 'no-store' },
    })
  }
  if (p.startsWith('/__lib/')) return lib(p.slice('/__lib/'.length))
  if (p.startsWith('/__raw/')) return raw(fromUrl(p, '/__raw'))
  if (p.startsWith('/__events/')) {
    const file = fromUrl(p, '/__events')
    return file ? events(file) : new Response('bad request', { status: 400 })
  }

  if (p.startsWith('/__notes/')) {
    const file = fromUrl(p, '/__notes')
    if (!file || !MARKDOWN.test(file)) return new Response('bad request', { status: 400 })
    if (req.method === 'GET') return getNotes(file)
    if (req.method === 'PUT') return putNotes(req, file)
    return new Response('method not allowed', { status: 405 })
  }

  if (p.startsWith('/__cursor/')) {
    if (req.method !== 'POST') return new Response('method not allowed', { status: 405 })
    const body = await jsonBody(req)
    const line = Number(body?.line)
    if (!Number.isInteger(line) || line < 1) return new Response('bad request', { status: 400 })
    const file = fromUrl(p, '/__cursor')
    if (!file) return new Response('bad request', { status: 400 })
    const w = entry(file)
    w.cursor = line
    send(w, frame('cursor', { line }))
    return new Response(null, { status: 204 })
  }

  if (p === '/') return new Response('md-preview: open a file with `md-preview <file.md>`\n')

  const path = fromUrl(p)
  if (!path) return new Response('bad request', { status: 400 })
  if (MARKDOWN.test(path)) return shell()
  return asset(req, path)
}

// ─── main ───────────────────────────────────────────────

const self = [join(HERE, 'server.js'), join(HERE, 'config.js')]
const boot = self.map(mtime).join()
let restarting = false

// launchd defers respawning a KeepAlive job that exited on its own
// ("pended nondemand spawn = inefficient") for minutes; a kickstart is immediate.
function restart() {
  restarting = true
  if (process.env.XPC_SERVICE_NAME !== LABEL) process.exit(0)
  Bun.spawn(['/bin/launchctl', 'kickstart', '-k', `gui/${process.getuid()}/${LABEL}`])
}

Bun.serve({ hostname: HOST, port: PORT, idleTimeout: 0, fetch: handle })
console.log(`md-preview: listening on http://${HOST}:${PORT} (pid ${process.pid})`)

// mtime polling, not fs.watch: an editor that writes atomically leaves an
// fs.watch handle bound to the replaced inode, silently, after the first save.
setInterval(() => {
  for (const [file, w] of watched) {
    if (!w.clients.size) continue
    const now = mtime(file)
    if (now !== w.mtime) {
      w.mtime = now
      send(w, frame('change', { gone: now === 0 }))
    }
  }
  if (!restarting && self.map(mtime).join() !== boot) restart()
}, POLL_MS)

setInterval(() => {
  for (const w of watched.values()) send(w, ': ping\n\n')
}, PING_MS)
