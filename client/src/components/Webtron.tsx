import { useEffect, useRef } from 'react'

import { playError, playWhoosh } from '@/audio/sound'
import Arena from '@/components/Arena'
import ArenaSelect from '@/components/ArenaSelect'
import Connect from '@/components/Connect'
import MainMenu from '@/components/MainMenu'
import useStore from '@/hooks/useStore'
import socketStates from '@/utils/socketStates'

import styles from './Webtron.module.css'

const stages = { MainMenu, Connect, ArenaSelect, Arena }

export default function Webtron() {
  const { stage, socketState } = useStore()
  useStageSounds(stage, socketState)

  const Stage = stages[stage] || null
  if (Stage === null) {
    const validStages = Object.keys(stages).join(', ')
    console.error(`No stage by name '${stage}' exists! Valid stages: ${validStages}`)
    return null
  }

  return (
    <div key={stage} className={styles.stage}>
      <Stage />
    </div>
  )
}

function useStageSounds(stage, socketState) {
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
    if (socketState === socketStates.CLOSED) playError()
  }, [socketState])
}
