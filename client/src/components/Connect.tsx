import useStore from '@/hooks/useStore'
import statusFromSocketState from '@/utils/statusFromSocketState'

import styles from './Connect.module.css'

export default function Connect() {
  const { socketState } = useStore()

  return (
    <div className={styles.connect}>
      <div className={styles.spinner}>
        <div className={styles.ring} />
        <div className={styles.ring} />
      </div>
      <div className={styles.statusText}>{statusFromSocketState(socketState)}</div>
    </div>
  )
}
