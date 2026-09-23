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

  return (
    <svg className={styles.lightribbon} viewBox={`0 0 ${width} ${height}`}>
      <polyline
        points={polylinePoints}
        stroke={colorToHexString(color)}
        strokeWidth="2"
        vectorEffect="non-scaling-stroke"
        shapeRendering="crispEdges"
        fill="none"
      />
    </svg>
  )
}
