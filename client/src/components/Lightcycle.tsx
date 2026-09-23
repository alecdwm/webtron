import useInterpolatedLightcyclePosition from '@/hooks/useInterpolatedLightcyclePosition'
import useStore from '@/hooks/useStore'
import { colorToHexString } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'

import styles from './Lightcycle.module.css'

type LightcycleProps = {
  name: string
  color: string
  position: [number, number]
  direction: string
  speed: number
  dead?: boolean
}

export default function Lightcycle({ name, color, position = [0, 0], direction, speed, dead }: LightcycleProps) {
  const {
    arena: { width, height },
  } = useStore()

  const interpolatedPosition = useInterpolatedLightcyclePosition(position, direction, speed, dead)

  const left = `${(interpolatedPosition[0] / width) * 100}%`
  const bottom = `${(interpolatedPosition[1] / height) * 100}%`
  const tagBottom = `calc(${(interpolatedPosition[1] / height) * 100}% + 20px)`
  const rotationMap = {
    up: '270deg',
    down: '90deg',
    left: '180deg',
    right: '0deg',
  }
  const transform = `translate(-50%, 50%) rotate(${rotationMap[direction]})`

  return (
    <>
      <div className={styles.nameTag} style={{ left, bottom: tagBottom, color: colorToHexString(color) }}>
        {name}
      </div>
      <img className={styles.lightcycle} src={lightcycleImages[color]} style={{ left, bottom, transform }} />
    </>
  )
}
