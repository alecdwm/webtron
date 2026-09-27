import { type CSSProperties, memo } from 'react'

import type { Color, NearMiss } from '@/types'
import { colorToCss } from '@/utils/colors'

import styles from './NearMisses.module.css'

type NearMissesProps = {
  nearMisses: NearMiss[]
  colorOf: (id: string) => Color
  selfId: string | null
  width: number
  height: number
}

// A spark where each near miss happened, and a word for the player's own,
// both bigger for a perfect one.
// Each plays its animation once and then stays hidden until it drops off the
// arena's list.
function NearMisses({ nearMisses, colorOf, selfId, width, height }: NearMissesProps) {
  return nearMisses.map(({ key, player, point: [x, y], perfect }) => {
    const style = {
      '--spark': colorToCss(colorOf(player)),
      left: `${(x / width) * 100}%`,
      bottom: `${(y / height) * 100}%`,
    } as CSSProperties
    return (
      <div key={key} className={styles.nearMiss} data-perfect={perfect || undefined} style={style}>
        <div className={styles.spark} />
        {player === selfId ? <div className={styles.label}>{perfect ? 'PERFECT!' : 'CLOSE!'}</div> : null}
      </div>
    )
  })
}

export default memo(NearMisses)
