import type { CSSProperties } from 'react'

import useInterpolatedLightcyclePosition from '@/hooks/useInterpolatedLightcyclePosition'
import useStore from '@/hooks/useStore'
import { colorToHexString } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'
import resolveClassName from '@/utils/resolveClassName'

import styles from './Lightcycle.module.css'

type LightcycleProps = {
  name: string
  color: string
  position: [number, number]
  direction: string
  speed: number
  dead?: boolean
  isSelf?: boolean
}

const rotationMap = {
  up: '270deg',
  down: '90deg',
  left: '180deg',
  right: '0deg',
}

export default function Lightcycle({
  name,
  color,
  position = [0, 0],
  direction,
  speed,
  dead,
  isSelf,
}: LightcycleProps) {
  const {
    arena: { width, height, started },
  } = useStore()

  const interpolatedPosition = useInterpolatedLightcyclePosition(position, direction, speed, dead)

  const left = `${(interpolatedPosition[0] / width) * 100}%`
  const bottom = `${(interpolatedPosition[1] / height) * 100}%`
  const tagBottom = `calc(${(interpolatedPosition[1] / height) * 100}% + 20px)`
  const transform = `translate(-50%, 50%) rotate(${rotationMap[direction]})`
  const colorVars = { '--cycle': colorToHexString(color) } as CSSProperties

  return (
    <>
      {isSelf && started === null && !dead ? (
        <div className={styles.selfMarker} style={{ ...colorVars, left, bottom }} />
      ) : null}
      <div
        className={resolveClassName([styles.nameTag, dead && styles.nameTagDead])}
        style={{ ...colorVars, left, bottom: tagBottom }}
      >
        {isSelf && started === null ? `${name} (you)` : name}
      </div>
      <img
        className={resolveClassName([styles.lightcycle, dead && styles.dead])}
        src={lightcycleImages[color]}
        style={{ ...colorVars, left, bottom, transform }}
        alt=""
      />
      {dead ? (
        <div className={styles.derez} style={{ ...colorVars, left, bottom }}>
          <div className={styles.ring} />
          <div className={styles.flash} />
          {[0, 1, 2, 3, 4, 5, 6, 7].map((index) => (
            <div key={index} className={styles.shard} style={{ '--angle': `${index * 45 + 20}deg` } as CSSProperties} />
          ))}
        </div>
      ) : null}
    </>
  )
}
