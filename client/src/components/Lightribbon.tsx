import useStore from '@/hooks/useStore'
import { colorToHexString } from '@/utils/colors'

import styles from './Lightribbon.module.css'

type LightribbonProps = {
  color: string
  paths: [number, number][][]
}

export default function Lightribbon({ color, paths }: LightribbonProps) {
  const {
    arena: { width, height },
  } = useStore()

  const d = paths.map((path) => `M${path.map((point) => `${point[0]},${width - point[1]}`).join('L')}`).join('')
  const stroke = colorToHexString(color)

  return (
    <svg className={styles.lightribbon} viewBox={`0 0 ${width} ${height}`}>
      {/* Soft halo, then the solid ribbon, then a bright core. */}
      <path
        className={styles.halo}
        d={d}
        stroke={stroke}
        strokeWidth="9"
        strokeLinejoin="round"
        strokeLinecap="round"
        vectorEffect="non-scaling-stroke"
        fill="none"
      />
      <path
        d={d}
        stroke={stroke}
        strokeWidth="3"
        strokeLinejoin="round"
        vectorEffect="non-scaling-stroke"
        fill="none"
      />
      <path
        className={styles.core}
        d={d}
        strokeWidth="1"
        vectorEffect="non-scaling-stroke"
        shapeRendering="crispEdges"
        fill="none"
      />
    </svg>
  )
}
