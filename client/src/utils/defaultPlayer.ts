import type { Player, PlayerSettings } from '@/types'

// The player as sent to the server, named "clu" if they left the name blank.
export default function defaultPlayer({ name, color }: PlayerSettings): Player {
  return { name: name || 'clu', color }
}
