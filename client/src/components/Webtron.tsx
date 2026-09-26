import { type ComponentType, useCallback, useEffect, useRef } from 'react'

import { disconnect, setStage } from '@/actions'
import { playError, playWhoosh } from '@/audio/sound'
import Arena from '@/components/Arena'
import ArenaSelect from '@/components/ArenaSelect'
import Connect from '@/components/Connect'
import ErrorBoundary from '@/components/ErrorBoundary'
import MainMenu from '@/components/MainMenu'
import useStore from '@/hooks/useStore'
import useStoreDispatch from '@/hooks/useStoreDispatch'
import type { SocketState, Stage as StageName } from '@/types'

import styles from './Webtron.module.css'

const stages: Record<StageName, ComponentType> = { MainMenu, Connect, ArenaSelect, Arena }

export default function Webtron() {
  const stage = useStore((store) => store.stage)
  const socketState = useStore((store) => store.socketState)
  useStageSounds(stage, socketState)
  const dispatch = useStoreDispatch()

  // A stage that fails to render drops the connection and returns to the main
  // menu, which says the connection closed.
  const onStageError = useCallback(() => {
    dispatch(disconnect())
    dispatch(setStage('MainMenu'))
  }, [dispatch])

  const Stage = stages[stage]

  return (
    <div key={stage} className={styles.stage}>
      <ErrorBoundary onError={onStageError}>
        <Stage />
      </ErrorBoundary>
    </div>
  )
}

function useStageSounds(stage: StageName, socketState: SocketState) {
  const previousStage = useRef(stage)
  useEffect(() => {
    if (previousStage.current === stage) return
    previousStage.current = stage
    playWhoosh()
  }, [stage])

  const previousSocketState = useRef(socketState)
  useEffect(() => {
    if (previousSocketState.current === socketState) return
    previousSocketState.current = socketState
    if (socketState === 'CLOSED') playError()
  }, [socketState])
}
