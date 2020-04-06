import socketStates from 'utils/socketStates'

export default function statusFromSocketState(socketState) {
  if (socketState === socketStates.CONNECTING) return 'CONNECTING'
  if (socketState === socketStates.CLOSED) return 'CONNECTION CLOSED'
  return ''
}
