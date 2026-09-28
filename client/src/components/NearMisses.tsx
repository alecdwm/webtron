import { type CSSProperties, memo } from 'react'

import useAnimationsDone from '@/hooks/useAnimationsDone'
import type { Color, NearMiss, Point } from '@/types'
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
// Each plays its animation once and is then taken out of the page, until it
// drops off the arena's list.
function NearMisses({ nearMisses, colorOf, selfId, width, height }: NearMissesProps) {
  return nearMisses.map(({ key, player, point, perfect }) => (
    <Spark
      key={key}
      color={colorOf(player)}
      point={point}
      perfect={perfect}
      self={player === selfId}
      width={width}
      height={height}
    />
  ))
}

export default memo(NearMisses)

type SparkProps = {
  color: Color
  point: Point
  perfect: boolean
  self: boolean
  width: number
  height: number
}

function Spark({ color, point: [x, y], perfect, self, width, height }: SparkProps) {
  // Both the spark and the word end faded out.
  const { done, onAnimationEnd } = useAnimationsDone(self ? 2 : 1)
  if (done) return null
  const style = {
    '--spark': colorToCss(color),
    left: `${(x / width) * 100}%`,
    bottom: `${(y / height) * 100}%`,
  } as CSSProperties
  return (
    <div className={styles.nearMiss} data-perfect={perfect || undefined} style={style} onAnimationEnd={onAnimationEnd}>
      <div className={styles.spark} />
      {self ? <div className={styles.label}>{perfect ? 'PERFECT!' : 'CLOSE!'}</div> : null}
    </div>
  )
}
