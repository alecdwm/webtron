import { type CSSProperties, memo } from 'react'

import type { Color, Point } from '@/types'
import { colorToCss } from '@/utils/colors'

import styles from './Hole.module.css'

type HoleProps = {
  color: Color
  position: Point
  radius: number
  width: number
  height: number
}

// The blast that clears the trails around an exploding lightcycle. It fades
// out after the explosion, and sits beneath the ribbons meanwhile so trails
// laid across it show on top.
function Hole({ color, position, radius, width, height }: HoleProps) {
  const style = {
    '--hole': colorToCss(color),
    left: `${(position[0] / width) * 100}%`,
    bottom: `${(position[1] / height) * 100}%`,
    width: `${((radius * 2) / width) * 100}%`,
  } as CSSProperties

  return (
    <div className={styles.hole} style={style}>
      <div className={styles.shockwave} />
    </div>
  )
}

export default memo(Hole)
