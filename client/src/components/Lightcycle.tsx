import { type CSSProperties, memo, useCallback } from 'react'

import { ghostOffsets, useArenaAnimationContext } from '@/hooks/useArenaAnimation'
import type { Color, Direction, EmoteShown, Point } from '@/types'
import { colorToCss } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'
import resolveClassName from '@/utils/resolveClassName'
import { emoteLabels } from '@/utils/round'

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
  // Whether the lightcycle is slowing under braking, and for the player's own
  // lightcycle in a round, how much brake it has left.
  braking: boolean
  brake: number | null
  // The player's latest emote, shown over the lightcycle as it comes in.
  emote: EmoteShown | undefined
  // Whether the arena wraps round, so the headlight shines through its edges.
  wrap: boolean
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
  braking,
  brake,
  emote,
  wrap,
}: LightcycleProps) {
  const animation = useArenaAnimationContext()
  const anchorRef = useCallback(
    (element: HTMLDivElement | null) => (element ? animation.trackLightcycle(id, element) : undefined),
    [animation, id],
  )

  const transform = `translate(-50%, 50%) rotate(${rotationMap[direction]})`
  const rezzing = spawning && !dead
  const colorVars = { '--cycle': colorToCss(color), '--spawn-delay': `${spawnDelay}ms` } as CSSProperties
  // The beam reaches further as the lightcycle speeds up, and pulls in as it
  // brakes.
  const beamVars = {
    '--beam-reach': Math.max(0.6, speed / LIGHTCYCLE_SPEED),
  } as CSSProperties
  const headlightClassName = resolveClassName([styles.headlight, rezzing && styles.fadeIn])
  const headlightStyle = { ...beamVars, transform: `rotate(${rotationMap[direction]})` }

  return (
    <>
      <div ref={anchorRef} className={styles.anchor} style={colorVars}>
        {isSelf && inLobby && !dead ? <div className={styles.selfMarker} /> : null}
        <div className={resolveClassName([styles.nameTag, dead && styles.nameTagDead, rezzing && styles.fadeIn])}>
          {isSelf && inLobby ? `${name} (you)` : name}
        </div>
        {!dead ? (
          <div className={headlightClassName} style={headlightStyle}>
            <div className={styles.beam} />
            {braking ? <div className={styles.brakeLight} /> : null}
          </div>
        ) : null}
        {brake !== null && !dead && (brake < 1 || braking) ? (
          <div className={styles.brakeMeter}>
            <div className={styles.brakeFill} style={{ transform: `scaleX(${brake})` }} />
          </div>
        ) : null}
        {emote ? (
          <div key={emote.key} className={styles.emote}>
            {emoteLabels[emote.emote]}
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
              <div
                key={index}
                className={styles.shard}
                style={{ '--angle': `${index * 45 + 20}deg` } as CSSProperties}
              />
            ))}
          </div>
        ) : null}
      </div>
      {wrap && !dead
        ? ghostOffsets.map((offset) => (
            <GhostLightcycle
              key={offset.join()}
              id={id}
              offset={offset}
              colorVars={colorVars}
              headlightClassName={headlightClassName}
              headlightStyle={headlightStyle}
              braking={braking}
              spriteClassName={resolveClassName([styles.lightcycle, rezzing && styles.spawning])}
              sprite={lightcycleImages[color]}
              spriteTransform={transform}
            />
          ))
        : null}
    </>
  )
}

export default memo(Lightcycle)

type GhostLightcycleProps = {
  id: string
  offset: Point
  colorVars: CSSProperties
  headlightClassName?: string
  headlightStyle: CSSProperties
  braking: boolean
  spriteClassName?: string
  sprite: string
  spriteTransform: string
}

// A copy of a lightcycle and its headlight a whole arena away, for an arena
// that wraps round, so a lightcycle riding through an edge, and its beam, come
// in at the opposite edge a bit at a time. The arena clips each copy to the
// part of it that has crossed back in. The arena animation hides copies
// that are nowhere near the arena.
function GhostLightcycle({
  id,
  offset,
  colorVars,
  headlightClassName,
  headlightStyle,
  braking,
  spriteClassName,
  sprite,
  spriteTransform,
}: GhostLightcycleProps) {
  const animation = useArenaAnimationContext()
  const ref = useCallback(
    (element: HTMLDivElement | null) => (element ? animation.trackGhost(id, element, offset) : undefined),
    [animation, id, offset],
  )
  return (
    <div ref={ref} className={styles.anchor} style={colorVars}>
      <div className={headlightClassName} style={headlightStyle}>
        <div className={styles.beam} />
        {braking ? <div className={styles.brakeLight} /> : null}
      </div>
      <img className={spriteClassName} src={sprite} style={{ transform: spriteTransform }} alt="" />
    </div>
  )
}
