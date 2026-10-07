export const PORT = Number(process.env.MD_PREVIEW_PORT) || 33440
export const HOST = '127.0.0.1'
export const ORIGIN = `http://${HOST}:${PORT}`
export const LABEL = 'com.github.gauthierdoppler.md-preview'
export const MARKDOWN = /\.(md|markdown)$/i

export function viewUrl(file) {
  return ORIGIN + file.split('/').map(encodeURIComponent).join('/')
}
