import { randomColor } from 'utils/colors'

export default function defaultPlayer(player) {
  if (!player.name) player.name = 'clu'
  if (!player.color) player.color = randomColor()
  return player
}
