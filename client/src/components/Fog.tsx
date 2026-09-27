import { useCallback } from 'react'

import { useArenaAnimationContext } from '@/hooks/useArenaAnimation'
import resolveClassName from '@/utils/resolveClassName'

import styles from './Fog.module.css'

// Darkness over the whole arena but a pool of light around the player's
// lightcycle, which the arena animation keeps centred on it. It lifts once
// the player is out, so they can watch the rest of the round.
export default function Fog({ lifted }: { lifted: boolean }) {
  const animation = useArenaAnimationContext()
  const ref = useCallback(
    (element: HTMLDivElement | null) => (element ? animation.trackFog(element) : undefined),
    [animation],
  )
  return <div ref={ref} className={resolveClassName([styles.fog, lifted && styles.lifted])} />
}
