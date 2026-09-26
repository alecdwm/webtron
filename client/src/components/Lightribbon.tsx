import { memo, useCallback, useMemo } from 'react'

import { useArenaAnimationContext } from '@/hooks/useArenaAnimation'
import type { Color, Point } from '@/types'
import { colorToHexString } from '@/utils/colors'
import resolveClassName from '@/utils/resolveClassName'
import { liveSegmentData, pathData } from '@/utils/ribbonPath'

import styles from './Lightribbon.module.css'

type CommonProps = {
  color: Color
  fixed: Point[][]
  width: number
  height: number
}
// A trail still in play. The arena animation draws its live segment, which
// follows the lightcycle.
type LiveProps = CommonProps & { id: string; derezzing?: false }
// A dead trail being animated out, drawn as it was left.
type DerezzingProps = CommonProps & { derezzing: true; tip: Point | null }
type LightribbonProps = LiveProps | DerezzingProps

// Each layer is drawn as two paths: the fixed part of the trail, which only
// changes when the lightcycle turns, and the live segment it is laying. The
// layers' opacity is set on their group, so the ends where the two paths
// overlap don't show brighter.
function Lightribbon(props: LightribbonProps) {
  const { color, fixed, width, height } = props
  const animation = useArenaAnimationContext()

  const fixedD = useMemo(() => pathData(fixed, height), [fixed, height])
  const staticLiveD = props.derezzing ? liveSegmentData(fixed, props.tip, null, height) : undefined
  const trackedId = props.derezzing ? null : props.id

  const svgRef = useCallback(
    (svg: SVGSVGElement | null) => {
      if (!svg || trackedId === null) return undefined
      return animation.trackLightribbon(trackedId, Array.from(svg.querySelectorAll<SVGPathElement>('path[data-live]')))
    },
    [animation, trackedId],
  )

  const layer = (
    <>
      <path d={fixedD} vectorEffect="non-scaling-stroke" />
      <path d={staticLiveD} data-live="" vectorEffect="non-scaling-stroke" />
    </>
  )

  return (
    <svg
      ref={svgRef}
      className={resolveClassName([styles.lightribbon, props.derezzing && styles.derez])}
      viewBox={`0 0 ${width} ${height}`}
    >
      {/* Soft halo, then the solid ribbon, then a bright core. */}
      <g
        className={styles.halo}
        stroke={colorToHexString(color)}
        strokeWidth="9"
        strokeLinejoin="round"
        strokeLinecap="round"
        fill="none"
      >
        {layer}
      </g>
      <g stroke={colorToHexString(color)} strokeWidth="3" strokeLinejoin="round" strokeLinecap="round" fill="none">
        {layer}
      </g>
      <g className={styles.core} strokeWidth="1" shapeRendering="crispEdges" fill="none">
        {layer}
      </g>
    </svg>
  )
}

export default memo(Lightribbon)
