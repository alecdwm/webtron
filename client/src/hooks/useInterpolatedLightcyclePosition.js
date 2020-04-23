import useRequestAnimationFrame from 'hooks/useRequestAnimationFrame'
import useStore from 'hooks/useStore'
import { useCallback, useRef, useState } from 'react'

export default function useInterpolatedLightcyclePosition(position, direction, speed, dead) {
  const {
    arena: { started },
  } = useStore()

  const basePosition = useRef(position)
  const basePositionSetAt = useRef(Date.now())

  const latestPosition = useRef(position)
  latestPosition.current = position

  const [interpolatedPosition, setInterpolatedPosition] = useState(position)

  const interpolate = useCallback(() => {
    if (!started || started.valueOf() > Date.now()) {
      if (started) basePositionSetAt.current = started.valueOf()
      return setInterpolatedPosition(latestPosition.current)
    }
    if (dead) return setInterpolatedPosition(latestPosition.current)

    if (basePosition.current !== latestPosition.current) {
      basePosition.current = latestPosition.current
      basePositionSetAt.current = Date.now()
    }

    const deltaTime = (Date.now() - basePositionSetAt.current) / 1000

    setInterpolatedPosition([
      basePosition.current[0] + (direction === 'left' ? -1 : direction === 'right' ? 1 : 0) * speed * deltaTime,
      basePosition.current[1] + (direction === 'down' ? -1 : direction === 'up' ? 1 : 0) * speed * deltaTime,
    ])
  }, [started, direction, speed, dead])

  useRequestAnimationFrame(interpolate)

  return interpolatedPosition
}
