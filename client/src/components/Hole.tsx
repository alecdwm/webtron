import type { CSSProperties } from 'react'

import useStore from '@/hooks/useStore'
import { colorToHexString } from '@/utils/colors'

import styles from './Hole.module.css'

type HoleProps = {
  color: string
  position: [number, number]
  radius: number
}

// The blast that clears the trails around an exploding lightcycle. It fades
// out after the explosion, and sits beneath the ribbons meanwhile so trails
// laid across it show on top.
export default function Hole({ color, position, radius }: HoleProps) {
  const {
    arena: { width, height },
  } = useStore()

  const style = {
    '--hole': colorToHexString(color),
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
