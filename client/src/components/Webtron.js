import Arena from 'components/Arena'
import ArenaSelect from 'components/ArenaSelect'
import Connect from 'components/Connect'
import MainMenu from 'components/MainMenu'
import useStore from 'hooks/useStore'
import React from 'react'

const stages = { MainMenu, Connect, ArenaSelect, Arena }

export default function Webtron() {
  const { stage } = useStore()

  const Stage = stages[stage] || null
  if (Stage === null) {
    const validStages = Object.keys(stages).join(', ')
    console.error(`No stage by name '${stage}' exists! Valid stages: ${validStages}`)
    return null
  }

  return <Stage />
}
