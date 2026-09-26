import { type CSSProperties, memo, useCallback } from 'react'

import { useArenaAnimationContext } from '@/hooks/useArenaAnimation'
import type { Color, Direction } from '@/types'
import { colorToCss } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'
import resolveClassName from '@/utils/resolveClassName'

import styles from './Lightcycle.module.css'

type LightcycleProps = {
  id: string
  name: string
  color: Color
  direction: Direction
  speed: number
  dead: boolean
  isSelf: boolean
  // True between rounds.
  inLobby: boolean
  // True from when the lightcycle is placed for a round until the round
  // starts, while it rezzes in.
  spawning: boolean
  // How long this lightcycle waits to rez in, to stagger a round's spawns.
  spawnDelay: number
}

// Base speed from src/server/arena.rs.
const LIGHTCYCLE_SPEED = 55

const rotationMap: Record<Direction, string> = {
  up: '270deg',
  down: '90deg',
  left: '180deg',
  right: '0deg',
}

// The lightcycle's position is set by the arena animation (see
// useArenaAnimation), not by rendering.
function Lightcycle({
  id,
  name,
  color,
  direction,
  speed,
  dead,
  isSelf,
  inLobby,
  spawning,
  spawnDelay,
}: LightcycleProps) {
  const animation = useArenaAnimationContext()
  const anchorRef = useCallback(
    (element: HTMLDivElement | null) => (element ? animation.trackLightcycle(id, element) : undefined),
    [animation, id],
  )

  const transform = `translate(-50%, 50%) rotate(${rotationMap[direction]})`
  const rezzing = spawning && !dead
  const colorVars = { '--cycle': colorToCss(color), '--spawn-delay': `${spawnDelay}ms` } as CSSProperties
  // The beam reaches further as the lightcycle speeds up.
  const beamVars = {
    '--beam-reach': 1 + Math.max(0, speed - LIGHTCYCLE_SPEED) / LIGHTCYCLE_SPEED,
  } as CSSProperties

  return (
    <div ref={anchorRef} className={styles.anchor} style={colorVars}>
      {isSelf && inLobby && !dead ? <div className={styles.selfMarker} /> : null}
      <div className={resolveClassName([styles.nameTag, dead && styles.nameTagDead, rezzing && styles.fadeIn])}>
        {isSelf && inLobby ? `${name} (you)` : name}
      </div>
      {!dead ? (
        <div
          className={resolveClassName([styles.headlight, rezzing && styles.fadeIn])}
          style={{ ...beamVars, transform: `rotate(${rotationMap[direction]})` }}
        >
          <div className={styles.beam} />
        </div>
      ) : null}
      <img
        className={resolveClassName([styles.lightcycle, dead && styles.dead, rezzing && styles.spawning])}
        src={lightcycleImages[color]}
        style={{ transform }}
        alt=""
      />
      {rezzing ? (
        <div className={styles.effect}>
          <div className={resolveClassName([styles.ring, styles.spawnRing])} />
          {[0, 1, 2, 3, 4, 5, 6, 7].map((index) => (
            <div
              key={index}
              className={resolveClassName([styles.shard, styles.spawnShard])}
              style={{ '--angle': `${index * 45}deg` } as CSSProperties}
            />
          ))}
          <div className={resolveClassName([styles.flash, styles.spawnFlash])} />
        </div>
      ) : null}
      {dead ? (
        <div className={styles.effect}>
          <div className={styles.ring} />
          <div className={styles.flash} />
          {[0, 1, 2, 3, 4, 5, 6, 7].map((index) => (
            <div key={index} className={styles.shard} style={{ '--angle': `${index * 45 + 20}deg` } as CSSProperties} />
          ))}
        </div>
      ) : null}
    </div>
  )
}

export default memo(Lightcycle)
