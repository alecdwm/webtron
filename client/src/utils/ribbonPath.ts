import type { Point } from '@/types'

// SVG path data for trail paths, in a viewBox the size of the arena. The
// arena's y axis points up and SVG's points down.
export function pathData(paths: Point[][], height: number): string {
  let d = ''
  for (const path of paths) {
    if (path.length === 0) continue
    d += `M${path.map(([x, y]) => `${x},${height - y}`).join('L')}`
  }
  return d
}

// SVG path data for the moving end of a trail: from its last fixed point to
// its tip, then on to `end` (where the lightcycle is drawn) if given.
export function liveSegmentData(fixed: Point[][], tip: Point | null, end: Point | null, height: number): string {
  const lastPath = fixed.length > 0 ? fixed[fixed.length - 1] : []
  const points: Point[] = []
  if (lastPath.length > 0) points.push(lastPath[lastPath.length - 1])
  if (tip) points.push(tip)
  if (end) points.push(end)
  return pathData([points], height)
}
