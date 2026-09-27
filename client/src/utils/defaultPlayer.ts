import type { Player, PlayerSettings } from '@/types'

// The player as sent to the server, named "user" if they left the name blank.
export default function defaultPlayer({ name, color }: PlayerSettings): Player {
  return { name: name || 'user', color }
}
