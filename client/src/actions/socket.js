import { receiveSocketMessage, setSocketState } from 'actions'
import defaultPlayer from 'utils/defaultPlayer'
import socketStates from 'utils/socketStates'

export let socket = null

export const CONNECT = 'CONNECT'
export const SEND = 'SEND'
export const GET_ARENA_LIST = 'GET_ARENA_LIST'
export const JOIN = 'JOIN'

export function connect() {
  return (dispatch) => {
    if (socket) return console.warn('cannot open new socket: socket already exists')

    const protocol = window.location.protocol === 'https' ? 'wss' : 'ws'
    const host = window.location.host
    const socket_url = `${protocol}://${host}/ws`

    socket = new WebSocket(socket_url)
    dispatch(setSocketState(socketStates.CONNECTING))

    socket.addEventListener('open', () => dispatch(setSocketState(socketStates.OPEN)))

    socket.addEventListener('message', (event) => {
      if (typeof event.data !== 'string') return console.warn('ignoring binary websocket message', event)

      const message = JSON.parse(event.data)
      const messageType = Object.keys(message).pop()
      const messageData = message[messageType]

      dispatch(receiveSocketMessage(messageType, messageData))
    })

    socket.addEventListener('error', (event) => {
      console.error('socket error', event)
      dispatch(setSocketState(socketStates.CLOSING))
    })
    socket.addEventListener('close', () => {
      dispatch(setSocketState(socketStates.CLOSED))
      socket = null
    })
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
