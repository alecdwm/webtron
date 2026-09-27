import { type CSSProperties, memo, useEffect, useRef, useState } from 'react'

import { playMutatorReveal } from '@/audio/sound'
import MutatorIcon, { ShapeIcon } from '@/components/MutatorIcon'
import type { Round } from '@/types'
import { serverNow } from '@/utils/clock'
import { colorToCss } from '@/utils/colors'
import { SHRINK_DELAY_SECONDS, mutatorInfo, shapeNames } from '@/utils/round'

import styles from './RoundBanner.module.css'

// The gap between one card coming in and the next.
const CARD_STAGGER_MS = 160

type RoundBannerProps = {
  round: Round
  // Unix time (ms) on the server's clock at which the round starts, or null
  // between rounds.
  startAt: number | null
  serverTimeOffset: number
}

// The round's shape and mutators: a card for each over the countdown, then a
// row of small icons in the corner while it is played.
function RoundBanner({ round, startAt, serverTimeOffset }: RoundBannerProps) {
  const started = useReachedTime(startAt, serverTimeOffset)
  if (startAt === null) return null
  if (round.shape === 'Grid' && round.mutators.length === 0) return null
  return started ? (
    <RoundHud key={startAt} round={round} startAt={startAt} serverTimeOffset={serverTimeOffset} />
  ) : (
    <RoundIntro key={startAt} round={round} />
  )
}

export default memo(RoundBanner)

function RoundIntro({ round }: { round: Round }) {
  const cards = [
    ...(round.shape !== 'Grid'
      ? [
          {
            key: 'shape',
            icon: <ShapeIcon className={styles.icon} blocked={round.blocked} />,
            name: shapeNames[round.shape],
            description: 'Arena',
            color: 'blue' as const,
          },
        ]
      : []),
    ...round.mutators.map((mutator) => ({
      key: mutator,
      icon: <MutatorIcon className={styles.icon} mutator={mutator} />,
      ...mutatorInfo[mutator],
    })),
  ]

  // Tracks whether the reveal was sounded, so StrictMode's repeated effect
  // runs don't sound it twice.
  const sounded = useRef(false)
  useEffect(() => {
    if (sounded.current) return
    sounded.current = true
    cards.forEach((_, index) => playMutatorReveal(index, (index * CARD_STAGGER_MS) / 1000))
  })

  return (
    <div className={styles.intro}>
      {cards.map(({ key, icon, name, description, color }, index) => (
        <div
          key={key}
          className={styles.card}
          style={{ '--card': colorToCss(color), animationDelay: `${index * CARD_STAGGER_MS}ms` } as CSSProperties}
        >
          {icon}
          <div className={styles.name}>{name}</div>
          <div className={styles.description}>{description}</div>
        </div>
      ))}
    </div>
  )
}

function RoundHud({ round, startAt, serverTimeOffset }: { round: Round; startAt: number; serverTimeOffset: number }) {
  // The Shrink icon pulses once the walls start closing in.
  const [shrinkDelayMs] = useState(() => startAt + SHRINK_DELAY_SECONDS * 1000 - serverNow(serverTimeOffset))
  return (
    <div className={styles.hud}>
      {round.shape !== 'Grid' ? (
        <span className={styles.hudItem} style={{ '--card': colorToCss('blue') } as CSSProperties}>
          <ShapeIcon className={styles.hudIcon} blocked={round.blocked} />
          <span className={styles.tooltip}>{shapeNames[round.shape]}</span>
        </span>
      ) : null}
      {round.mutators.map((mutator) => (
        <span
          key={mutator}
          className={mutator === 'Shrink' ? styles.hudItemShrinking : styles.hudItem}
          style={
            {
              '--card': colorToCss(mutatorInfo[mutator].color),
              animationDelay: mutator === 'Shrink' ? `${shrinkDelayMs}ms` : undefined,
            } as CSSProperties
          }
        >
          <MutatorIcon className={styles.hudIcon} mutator={mutator} />
          <span className={styles.tooltip}>{mutatorInfo[mutator].name}</span>
        </span>
      ))}
    </div>
  )
}

// True once server time `at` has passed, until `at` changes or becomes null.
function useReachedTime(at: number | null, serverTimeOffset: number) {
  const [reached, setReached] = useState<number | null>(null)

  useEffect(() => {
    if (at === null) return
    const delay = Math.max(0, at - serverNow(serverTimeOffset))
    const timeout = window.setTimeout(() => setReached(at), delay)
    return () => window.clearTimeout(timeout)
  }, [at, serverTimeOffset])

  return at !== null && reached === at
}
