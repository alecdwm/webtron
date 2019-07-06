import { useRef, useCallback } from 'react'
import { receiveSocketMessage, setGameState, setSocketState, setStatusText } from 'actions'
import useForceUpdate from 'hooks/useForceUpdate'
import useStoreDispatch from 'hooks/useStoreDispatch'

export const SocketStates = {
  NOT_CONNECTED: 'NOT_CONNECTED',
  CONNECTING: 'CONNECTING',
  OPEN: 'OPEN',
  CLOSING: 'CLOSING',
  CLOSED: 'CLOSED',
}

export default function useSocket() {
  const dispatch = useStoreDispatch()
  const socketRef = useRef(null)
  const forceUpdate = useForceUpdate()

  const connect = useCallback(() => {
    if (socketRef.current) return console.warn('cannot open new socket: socket already exists')

    const protocol = window.location.protocol === 'https' ? 'wss' : 'ws'
    const socket_url = `${protocol}://${window.location.host}/ws`

    socketRef.current = new WebSocket(socket_url)
    dispatch(setSocketState(SocketStates.CONNECTING))

    socketRef.current.addEventListener('open', () => {
      dispatch(setStatusText(''))
      dispatch(setSocketState(SocketStates.OPEN))
    })

    socketRef.current.addEventListener('message', event => {
      if (typeof event.data !== 'string') return console.warn('ignoring binary websocket message', event)

      const message = JSON.parse(event.data)
      const messageType = Object.keys(message).pop()
      const messageData = message[messageType]

      dispatch(receiveSocketMessage(messageType, messageData))
    })

    socketRef.current.addEventListener('error', event => {
      console.error('socket error', event)

      dispatch(setStatusText('CONNECTION ERROR'))
      dispatch(setSocketState(SocketStates.CLOSING))
    })

    socketRef.current.addEventListener('close', () => {
      dispatch(setStatusText('CONNECTION CLOSED'))
      dispatch(setSocketState(SocketStates.CLOSED))
      socketRef.current = null
      forceUpdate()
      dispatch(setGameState('MainMenu'))
    })
  }, [dispatch, socketRef, forceUpdate])

  const disconnect = useCallback(() => {
    if (!socketRef.current) return console.warn('cannot close socket: socket does not exist')
    socketRef.current.close()
  }, [socketRef])

  const send = useCallback(
    message => {
      if (!socketRef.current) return console.warn('cannot send socket message: socket not found')
      socketRef.current.send(JSON.stringify(message))
    },
    [socketRef],
  )

  return [connect, disconnect, send]
}
