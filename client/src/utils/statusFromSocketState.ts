import type { SocketState } from '@/types'

export default function statusFromSocketState(socketState: SocketState): string {
  if (socketState === 'CONNECTING') return 'CONNECTING'
  if (socketState === 'CLOSED') return 'CONNECTION CLOSED'
  return ''
}
