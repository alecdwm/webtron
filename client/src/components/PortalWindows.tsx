import { type CSSProperties, memo, useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { useArenaAnimationContext } from '@/hooks/useArenaAnimation'
import type { Color, Lightribbon, Round } from '@/types'
import { colorToCss } from '@/utils/colors'
import { gatesOf } from '@/utils/round'
import { type Crossing, crossingsOf, gatesToSee } from '@/utils/views'

import styles from './PortalWindows.module.css'

// How long a window takes to close once the trail through it is gone.
const CLOSE_MS = 600
// How long a window is along the portal, in arena units: about as wide as a
// lightcycle.
const WINDOW_LENGTH = 12

type PortalWindowsProps = {
  lightribbons: Record<string, Lightribbon>
  round: Round
  width: number
  height: number
  colorOf: (id: string) => Color
}

// A window in each portal of a pair that a trail goes through, in the trail's
// colour, where it went in and where it came out of the twin. A window opens
// as the lightcycle passes through, stays open while its trail is there on
// both sides, and closes once the trail is gone from either side: when it is
// trimmed away or cut, or derezzes after the lightcycle crashes.
function PortalWindows({ lightribbons, round, width, height, colorOf }: PortalWindowsProps) {
  const crossings = useMemo(
    () => (width === 0 ? [] : crossingsOf(lightribbons, gatesToSee(gatesOf(round, width, height)))),
    [lightribbons, round, width, height],
  )
  const closing = useClosing(crossings)

  const windows = [
    ...crossings.map((crossing) => ({ crossing, closing: false })),
    ...closing.map((crossing) => ({ crossing, closing: true })),
  ]
  return (
    <>
      {windows.flatMap(({ crossing: { key, player, entry, exit, upright }, closing }) =>
        [entry, exit].map(([x, y], side) => {
          const length = `${(WINDOW_LENGTH / (upright ? height : width)) * 100}%`
          const style = {
            left: `${(x / width) * 100}%`,
            bottom: `${(y / height) * 100}%`,
            [upright ? 'height' : 'width']: length,
            '--cycle': colorToCss(colorOf(player)),
          } as CSSProperties
          const classes = [styles.window, !upright && styles.across, closing && styles.closing]
          return <div key={`${key}-${side}`} className={classes.filter(Boolean).join(' ')} style={style} />
        }),
      )}
    </>
  )
}

export default memo(PortalWindows)

// A window a lightcycle opens in a portal while any of its sprite touches it,
// where it touches, or the same place on the twin. The arena animation places
// it, sizes it to the stretch touched and hides it the rest of the time; it
// opens afresh each time it is shown.
export function TouchWindow({ id, twin, color }: { id: string; twin: boolean; color: Color }) {
  const animation = useArenaAnimationContext()
  const ref = useCallback(
    (element: HTMLDivElement | null) => (element ? animation.trackTouch(id, element, twin, styles.across) : undefined),
    [animation, id, twin],
  )
  return (
    <div
      ref={ref}
      className={styles.window}
      style={{ display: 'none', '--cycle': colorToCss(color) } as CSSProperties}
    />
  )
}

// The crossings that were in `crossings` until lately, while their windows
// close.
function useClosing(crossings: Crossing[]) {
  const [closing, setClosing] = useState<Crossing[]>([])
  const previous = useRef<Crossing[]>([])
  const timers = useRef(new Set<ReturnType<typeof setTimeout>>())

  useEffect(() => {
    const open = new Set(crossings.map(({ key }) => key))
    const gone = previous.current.filter(({ key }) => !open.has(key))
    previous.current = crossings
    if (gone.length === 0) return
    // A window reopened by a trail going through the same place again is
    // open, not closing.
    setClosing((closing) => [...closing.filter(({ key }) => !open.has(key)), ...gone])
    const timer = setTimeout(() => {
      timers.current.delete(timer)
      setClosing((closing) => closing.filter((crossing) => !gone.includes(crossing)))
    }, CLOSE_MS)
    timers.current.add(timer)
  }, [crossings])

  useEffect(() => {
    const pending = timers.current
    return () => pending.forEach(clearTimeout)
  }, [])

  return closing
}
