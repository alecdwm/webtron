import type { ClientMessage } from '@/types'

// The connection to the server, if any. There is at most one at a time.
let socket: WebSocket | null = null

type SocketListeners = {
  open: () => void
  message: (data: string) => void
  error: () => void
  close: () => void
}

// Opens a connection to the server unless one is already open. Events from a
// socket that has since been replaced or closed are ignored.
export function openSocket(listeners: SocketListeners): boolean {
  if (socket) {
    console.warn('cannot open new socket: socket already exists')
    return false
  }

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const current = new WebSocket(`${protocol}//${window.location.host}/ws`)
  socket = current

  current.addEventListener('open', () => {
    if (socket === current) listeners.open()
  })
  current.addEventListener('message', (event) => {
    if (socket !== current) return
    if (typeof event.data !== 'string') return console.warn('ignoring binary websocket message', event)
    listeners.message(event.data)
  })
  current.addEventListener('error', (event) => {
    if (socket !== current) return
    console.error('socket error', event)
    listeners.error()
  })
  current.addEventListener('close', () => {
    if (socket !== current) return
    socket = null
    listeners.close()
  })
  return true
}

// Closes the connection without waiting for the close to complete. Returns
// whether there was one to close.
export function closeSocket(): boolean {
  if (!socket) return false
  const closing = socket
  socket = null
  closing.close()
  return true
}

export function send(message: ClientMessage) {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    return console.warn('cannot send socket message: socket not open', message)
  }
  socket.send(JSON.stringify(message))
}
