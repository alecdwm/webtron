import { type CSSProperties, memo, type ReactNode, useCallback } from 'react'

import { useArenaAnimationContext } from '@/hooks/useArenaAnimation'
import type { Color, Direction, EmoteShown } from '@/types'
import { colorToCss } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'
import resolveClassName from '@/utils/resolveClassName'
import { emoteLabels } from '@/utils/round'
import { MAX_COPIES } from '@/utils/views'

import styles from './Lightcycle.module.css'
import { TouchWindow } from './PortalWindows'

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
  // The player's latest emote, if it's recent enough to still show over the
  // lightcycle.
  emote: EmoteShown | undefined
  // Whether the arena wraps round or has portals, so the lightcycle, its
  // headlight and its name can be seen through them, and whether it has
  // portals, which the lightcycle opens a window in while it touches one.
  copies: boolean
  portals: boolean
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
  copies,
  portals,
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

  const nameTag = (
    <div className={resolveClassName([styles.nameTag, dead && styles.nameTagDead, rezzing && styles.fadeIn])}>
      {isSelf && inLobby ? `${name} (you)` : name}
    </div>
  )
  const headlight = !dead ? (
    <div className={headlightClassName} style={headlightStyle}>
      <div className={styles.beam} />
      {braking ? <div className={styles.brakeLight} /> : null}
    </div>
  ) : null
  const spriteClassName = resolveClassName([styles.lightcycle, dead && styles.dead, rezzing && styles.spawning])
  const sprite = <img className={spriteClassName} src={lightcycleImages[color]} style={{ transform }} alt="" />

  // The headlight and the sprite each have their own part, which the arena
  // animation clips separately where they are seen through a gate (see
  // utils/views.ts). The name and the emote bubble aren't clipped: through a
  // portal they stay with the lightcycle itself, and only copies seen through
  // the edges of an arena that wraps round show them too.
  return (
    <>
      <div ref={anchorRef} className={styles.anchor} style={colorVars}>
        {isSelf && inLobby && !dead ? <div className={styles.selfMarker} /> : null}
        <div className={styles.namePart} data-part="name">
          {nameTag}
        </div>
        <div className={styles.bodyPart} data-part="beam">
          <div className={styles.partOrigin}>{headlight}</div>
        </div>
        <div className={styles.bodyPart} data-part="sprite">
          <div className={styles.partOrigin}>{sprite}</div>
        </div>
        {brake !== null && !dead && (brake < 1 || braking) ? (
          <div className={styles.brakeMeter}>
            <div className={styles.brakeFill} style={{ transform: `scaleX(${brake})` }} />
          </div>
        ) : null}
        {emote ? <EmoteBubble key={emote.key} id={id} emote={emote} ghost={false} /> : null}
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
      {copies && !dead
        ? Array.from({ length: MAX_COPIES }, (_, index) => (
            <Copy key={index} id={id} index={index} colorVars={colorVars} emote={emote}>
              {nameTag}
              {headlight}
              {sprite}
            </Copy>
          ))
        : null}
      {portals && !dead ? (
        <>
          <TouchWindow id={id} twin={false} color={color} />
          <TouchWindow id={id} twin color={color} />
        </>
      ) : null}
    </>
  )
}

export default memo(Lightcycle)

type CopyProps = {
  id: string
  index: number
  colorVars: CSSProperties
  emote: EmoteShown | undefined
  // The name tag, headlight and sprite, in that order.
  children: [ReactNode, ReactNode, ReactNode]
}

// A copy of a lightcycle and its headlight, seen through a portal or an edge
// of an arena that wraps round. The arena animation places each of a
// lightcycle's copies where the next of its views puts it, clips its parts to
// where the view sees them, and hides copies it has no view for. The name and
// emote bubble are only shown on copies seen through edges alone.
function Copy({ id, index, colorVars, emote, children: [nameTag, headlight, sprite] }: CopyProps) {
  const animation = useArenaAnimationContext()
  const ref = useCallback(
    (element: HTMLDivElement | null) => (element ? animation.trackCopy(id, element, index) : undefined),
    [animation, id, index],
  )
  return (
    <div ref={ref} className={styles.anchor} style={{ ...colorVars, display: 'none' }}>
      <div className={styles.namePart} data-part="name">
        {nameTag}
      </div>
      <div className={styles.bodyPart} data-part="beam">
        <div className={styles.partOrigin}>{headlight}</div>
      </div>
      <div className={styles.bodyPart} data-part="sprite">
        <div className={styles.partOrigin}>{sprite}</div>
      </div>
      <div data-part="emote">{emote ? <EmoteBubble key={emote.key} id={id} emote={emote} ghost /> : null}</div>
    </div>
  )
}

// A speech bubble that pops up over the lightcycle, then floats off. The
// arena animation plays it and moves it to stay in view.
function EmoteBubble({ id, emote, ghost }: { id: string; emote: EmoteShown; ghost: boolean }) {
  const animation = useArenaAnimationContext()
  const ref = useCallback(
    (element: HTMLDivElement | null) => (element ? animation.trackEmote(id, element, emote.at, ghost) : undefined),
    [animation, id, emote.at, ghost],
  )
  return (
    <div ref={ref} className={styles.emote}>
      {emoteLabels[emote.emote]}
    </div>
  )
}
