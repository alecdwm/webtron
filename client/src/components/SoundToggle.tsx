import { useCallback, useSyncExternalStore } from 'react'
import { createPortal } from 'react-dom'

import { isMuted, setMuted, subscribeMuted, unlockAudio } from '@/audio/sound'
import useEventListener from '@/hooks/useEventListener'

import styles from './SoundToggle.module.css'

export default function SoundToggle() {
  const muted = useSyncExternalStore(subscribeMuted, isMuted)

  const toggle = useCallback(() => {
    unlockAudio()
    setMuted(!isMuted())
  }, [])

  const onKeyDown = useCallback(
    (event) => {
      if (event.key !== 'm' || event.target instanceof HTMLInputElement) return
      toggle()
    },
    [toggle],
  )
  useEventListener('keydown', onKeyDown)

  return createPortal(
    <button
      className={styles.soundToggle}
      onClick={toggle}
      aria-label={muted ? 'Unmute sound' : 'Mute sound'}
      title={muted ? 'Unmute (M)' : 'Mute (M)'}
    >
      <svg width="1em" height="1em" viewBox="0 0 16 16" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
        <path d="M2 5.5h2.5L8 2.5v11l-3.5-3H2z" />
        {muted ? (
          <path d="M10.5 5.5l4 5m0-5l-4 5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
        ) : (
          <path
            d="M10.5 5.5a3.5 3.5 0 010 5M12.5 3.5a6.5 6.5 0 010 9"
            stroke="currentColor"
            strokeWidth="1.4"
            strokeLinecap="round"
            fill="none"
          />
        )}
      </svg>
    </button>,
    document.body,
  )
}
