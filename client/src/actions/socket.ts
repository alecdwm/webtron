import { receiveSocketMessage, setSocketState } from '@/actions'
import defaultPlayer from '@/utils/defaultPlayer'
import socketStates from '@/utils/socketStates'

export let socket = null

export const CONNECT = 'CONNECT'
export const SEND = 'SEND'
export const GET_ARENA_LIST = 'GET_ARENA_LIST'
export const JOIN = 'JOIN'
export const START = 'START'
export const TURN = 'TURN'

export function connect() {
  return (dispatch, getState) => {
    if (socket) return console.warn('cannot open new socket: socket already exists')

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const host = window.location.host
    const socket_url = `${protocol}//${host}/ws`

    // Events from a socket that has since been replaced or disconnected are
    // ignored.
    const current = new WebSocket(socket_url)
    socket = current
    dispatch(setSocketState(socketStates.CONNECTING))

    current.addEventListener('open', () => {
      if (socket !== current) return
      // Claim the player's name first, so a name already in use is refused
      // before anything else happens.
      dispatch(send({ ClaimName: defaultPlayer(getState().player).name }))
      dispatch(setSocketState(socketStates.OPEN))
    })

    current.addEventListener('message', (event) => {
      if (socket !== current) return
      if (typeof event.data !== 'string') return console.warn('ignoring binary websocket message', event)

      const message = JSON.parse(event.data)
      const messageType = typeof message === 'object' ? Object.keys(message).pop() : message
      const messageData = typeof message === 'object' ? message[messageType] : null

      dispatch(receiveSocketMessage(messageType, messageData))
    })

    current.addEventListener('error', (event) => {
      if (socket !== current) return
      console.error('socket error', event)
      dispatch(setSocketState(socketStates.CLOSING))
    })
    current.addEventListener('close', () => {
      if (socket !== current) return
      socket = null
      dispatch(setSocketState(socketStates.CLOSED))
    })
  }
}

// Closes the socket and returns to the main menu straight away, without
// waiting for the close to complete.
export function disconnect() {
  return (dispatch) => {
    if (!socket) return
    const closing = socket
    socket = null
    closing.close()
    dispatch(setSocketState(socketStates.CLOSED))
  }
}

export function send(message) {
  return (dispatch) => {
    if (!socket) return console.warn('cannot send socket message: socket not found')
    dispatch({ type: SEND, message })
    socket.send(JSON.stringify(message))
  }
}

export function getArenaList() {
  return (dispatch) => {
    dispatch({ type: GET_ARENA_LIST })
    dispatch(send({ GetArenaList: null }))
  }
}

export function join(arenaId = null) {
  return (dispatch, getState) => {
    const { player } = getState()
    dispatch({ type: JOIN, player, arenaId })
    dispatch(send({ Join: { player: defaultPlayer(player), arena_id: arenaId } }))
  }
}

export function start() {
  return (dispatch) => {
    dispatch({ type: START })
    dispatch(send({ Start: null }))
  }
}

export function turn(direction) {
  return (dispatch) => {
    dispatch({ type: TURN, direction })
    dispatch(send({ Turn: direction }))
  }
}
