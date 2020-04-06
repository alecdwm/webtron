import { SocketStates } from 'hooks/useSocket'

export default function statusFromSocketState(socketState) {
  if (socketState === SocketStates.CONNECTING) return 'CONNECTING'
  if (socketState === SocketStates.CLOSED) return 'CONNECTION CLOSED'
  return ''
}
