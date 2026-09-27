import type { ReactNode } from 'react'

import type { Mutator } from '@/types'
import { LAYOUT_CELLS, wallOutline } from '@/utils/round'

// Line drawings on a 24 by 24 grid, stroked in the current colour.
const icons: Record<Mutator, ReactNode> = {
  // Arrows closing in from each corner on a small square.
  Shrink: (
    <>
      <rect x="9" y="9" width="6" height="6" rx="1" />
      <path d="M3 3l4 4M7 3.5V7H3.5" />
      <path d="M21 3l-4 4M17 3.5V7h3.5" />
      <path d="M3 21l4-4M7 20.5V17H3.5" />
      <path d="M21 21l-4-4M17 20.5V17h3.5" />
    </>
  ),
  // A trail leaving by the right edge and coming back in at the left.
  Wrap: (
    <>
      <path d="M3 4v16M21 4v16" strokeDasharray="2 2.5" strokeWidth="1.5" />
      <path d="M14 12h7" />
      <path d="M3 12h7M7.5 9l3 3-3 3" />
    </>
  ),
  // A short trail behind a lightcycle, fading towards its tail.
  Snake: (
    <>
      <path d="M3 18h4" strokeOpacity="0.35" />
      <path d="M7 18h4v-8" strokeOpacity="0.7" />
      <path d="M11 10h6" />
      <circle cx="19" cy="10" r="2" fill="currentColor" stroke="none" />
    </>
  ),
  // Double chevron with speed lines.
  Overdrive: (
    <>
      <path d="M7 6l6 6-6 6M13 6l6 6-6 6" />
      <path d="M2 9h3M1 12h4M2 15h3" strokeWidth="1.5" />
    </>
  ),
  // A pool of light surrounded by dark.
  Fog: (
    <>
      <circle cx="12" cy="12" r="3" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="6.5" strokeOpacity="0.6" />
      <circle cx="12" cy="12" r="10" strokeDasharray="1.5 3" strokeOpacity="0.35" />
    </>
  ),
  // Blocks scattered over the grid.
  Obstacles: (
    <>
      <rect x="3.5" y="4" width="5" height="5" rx="0.5" />
      <rect x="14" y="7.5" width="5" height="5" rx="0.5" />
      <rect x="6.5" y="15" width="5" height="5" rx="0.5" />
    </>
  ),
}

export default function MutatorIcon({ mutator, className }: { mutator: Mutator; className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {icons[mutator]}
    </svg>
  )
}

// A map of the arena's walled-off cells, with the arena's edge around them.
export function ShapeIcon({ blocked, className }: { blocked: [number, number][]; className?: string }) {
  const outline = wallOutline(blocked)
  return (
    <svg
      className={className}
      viewBox={`-1 -1 ${LAYOUT_CELLS + 2} ${LAYOUT_CELLS + 2}`}
      fill="none"
      stroke="currentColor"
      strokeLinecap="square"
      aria-hidden="true"
    >
      <rect x="0" y="0" width={LAYOUT_CELLS} height={LAYOUT_CELLS} rx="1" strokeWidth="1" strokeOpacity="0.5" />
      {blocked.map(([x, y]) => (
        <rect
          key={`${x},${y}`}
          x={x}
          y={LAYOUT_CELLS - 1 - y}
          width="1"
          height="1"
          fill="currentColor"
          fillOpacity="0.35"
          stroke="none"
        />
      ))}
      <path
        d={outline.map(([x1, y1, x2, y2]) => `M${x1} ${LAYOUT_CELLS - y1}L${x2} ${LAYOUT_CELLS - y2}`).join('')}
        strokeWidth="0.8"
      />
    </svg>
  )
}
