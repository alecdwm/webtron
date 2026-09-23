import { useEffect, useRef, useState } from 'react'

import { playCountdownBeep, playCountdownGo } from '@/audio/sound'

import styles from './Countdown.module.css'

const GO_DURATION_MS = 900

type CountdownProps = {
  // Unix timestamp (ms) at which the round starts.
  startAt: number
}

export default function Countdown({ startAt }: CountdownProps) {
  const [now, setNow] = useState(() => Date.now())

  const finished = now >= startAt + GO_DURATION_MS
  useEffect(() => {
    if (finished) return
    const interval = window.setInterval(() => setNow(Date.now()), 50)
    return () => window.clearInterval(interval)
  }, [finished])

  const remaining = Math.ceil((startAt - now) / 1000)
  const text = remaining > 0 ? String(remaining) : 'GO!'

  // Tracks the last text a sound was played for, so StrictMode's repeated
  // effect runs don't play it twice.
  const soundedText = useRef<string | null>(null)
  useEffect(() => {
    if (finished || soundedText.current === text) return
    soundedText.current = text
    if (text === 'GO!') playCountdownGo()
    else playCountdownBeep()
  }, [text, finished])

  if (finished) return null

  return (
    <div className={styles.countdown}>
      <div key={text} className={remaining > 0 ? styles.number : styles.go}>
        {text}
      </div>
    </div>
  )
}
