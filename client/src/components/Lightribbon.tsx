import useStore from '@/hooks/useStore'
import { colorToHexString } from '@/utils/colors'

import styles from './Lightribbon.module.css'

type LightribbonProps = {
  color: string
  points: [number, number][]
}

export default function Lightribbon({ color, points }: LightribbonProps) {
  const {
    arena: { width, height },
  } = useStore()

  const polylinePoints = points.map((point) => `${point[0]}, ${width - point[1]}`).join(' ')
  const stroke = colorToHexString(color)

  return (
    <svg className={styles.lightribbon} viewBox={`0 0 ${width} ${height}`}>
      {/* Soft halo, then the solid ribbon, then a bright core. */}
      <polyline
        className={styles.halo}
        points={polylinePoints}
        stroke={stroke}
        strokeWidth="9"
        strokeLinejoin="round"
        strokeLinecap="round"
        vectorEffect="non-scaling-stroke"
        fill="none"
      />
      <polyline
        points={polylinePoints}
        stroke={stroke}
        strokeWidth="3"
        strokeLinejoin="round"
        vectorEffect="non-scaling-stroke"
        fill="none"
      />
      <polyline
        className={styles.core}
        points={polylinePoints}
        strokeWidth="1"
        vectorEffect="non-scaling-stroke"
        shapeRendering="crispEdges"
        fill="none"
      />
    </svg>
  )
}
