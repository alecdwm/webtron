import useClassName from 'hooks/useClassName'
import useStore from 'hooks/useStore'
import React from 'react'
import statusFromSocketState from 'utils/statusFromSocketState'

import styles from './Connect.module.css'

export default function Connect() {
  const { socketState } = useStore()

  const StatusText = useClassName(styles.statusText)

  return <StatusText>{statusFromSocketState(socketState)}</StatusText>
}
