import { type AnimationEvent, useCallback, useRef, useState } from 'react'

// Whether `count` CSS animations in an element have ended, for effects that
// play once and end invisible, so they can be taken out of the page instead
// of being kept there drawn as nothing. `onAnimationEnd` goes on the element,
// and counts the ends of its descendants' animations as they bubble up.
export default function useAnimationsDone(count: number) {
  const ended = useRef(0)
  const [done, setDone] = useState(false)
  const onAnimationEnd = useCallback(
    (_event: AnimationEvent) => {
      ended.current += 1
      if (ended.current >= count) setDone(true)
    },
    [count],
  )
  return { done, onAnimationEnd }
}
