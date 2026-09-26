import { createPortal } from 'react-dom'

import styles from './Floor.module.css'

// Rendered into the body rather than the app's <main>, whose warm-up
// animation uses a transform and a filter. While that runs, <main> would be
// the containing block for the floor's fixed position, instead of the
// viewport.
export default function Floor() {
  return createPortal(
    <div className={styles.floor} aria-hidden="true">
      <div className={styles.grid} />
    </div>,
    document.body,
  )
}
