import { useCallback, useId } from 'react'

import { useArenaAnimationContext } from '@/hooks/useArenaAnimation'
import resolveClassName from '@/utils/resolveClassName'

import styles from './Fog.module.css'

// Darkness over the whole arena but pools of light round the player's
// lightcycle, and round copies of it seen through portals and wrapping
// edges, which the arena animation draws into the mask's pools group in
// arena units. Each pool is a circle filled with the pool gradient, which
// takes the fog away in the middle and fades it back in towards the edge;
// where pools overlap, what each leaves of the fog multiplies together. It
// lifts once the player is out, so they can watch the rest of the round.
export default function Fog({ lifted }: { lifted: boolean }) {
  const animation = useArenaAnimationContext()
  // useId's ids can hold characters that break a url(#…) reference.
  const id = `fog${useId().replace(/[^\w-]/g, '')}`
  const ref = useCallback(
    (element: SVGGElement | null) => (element ? animation.trackFog(element, id) : undefined),
    [animation, id],
  )
  return (
    <svg
      className={resolveClassName([styles.fog, lifted && styles.lifted])}
      viewBox="0 0 1 1"
      preserveAspectRatio="none"
      aria-hidden
    >
      <defs>
        <radialGradient id={`${id}-pool`}>
          <stop offset={12 / 26} stopOpacity={1} />
          <stop offset={18 / 26} stopOpacity={0.23} />
          <stop offset={1} stopOpacity={0} />
        </radialGradient>
        <mask id={`${id}-mask`} maskUnits="userSpaceOnUse" x={0} y={0} width={1} height={1} colorInterpolation="sRGB">
          <rect width={1} height={1} fill="#fff" />
          <g ref={ref} />
        </mask>
      </defs>
      <rect className={styles.darkness} width={1} height={1} mask={`url(#${id}-mask)`} />
    </svg>
  )
}
