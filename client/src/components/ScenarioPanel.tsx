import type { ReactNode } from 'react'

import type { Mutator, NpcStyle, Scenario, Shape } from '@/types'
import { mutatorInfo, shapeNames } from '@/utils/round'

import styles from './ScenarioPanel.module.css'

const shapes = Object.keys(shapeNames) as Shape[]
const mutators = Object.keys(mutatorInfo) as Mutator[]
const npcStyles: NpcStyle[] = ['Hunter', 'Racer', 'Trickster', 'Sentinel', 'Shadow', 'Boxer', 'Kamikaze', 'Baiter']
// The most NPCs a round can have, alongside one human player.
const MAX_NPCS = 7

type ScenarioPanelProps = {
  scenario: Scenario
  onChange: (scenario: Scenario) => void
  onClose: () => void
}

// Sets up the next round for testing: its shape, its mutators and its NPCs'
// styles, each rolled at random as usual unless picked. The arena opens it
// from a hidden gesture (see Arena).
export default function ScenarioPanel({ scenario, onChange, onClose }: ScenarioPanelProps) {
  const toggleMutator = (mutator: Mutator) => {
    const picked = scenario.mutators ?? []
    onChange({
      ...scenario,
      mutators: picked.includes(mutator) ? picked.filter((other) => other !== mutator) : [...picked, mutator],
    })
  }
  const npcs = scenario.npcs ?? []

  return (
    <div className={styles.panel}>
      <div className={styles.heading}>
        Test scenario
        <button type="button" className={styles.close} onClick={onClose}>
          Close
        </button>
      </div>

      <div className={styles.row}>
        <span className={styles.label}>Shape</span>
        <Chip on={scenario.shape === null} onClick={() => onChange({ ...scenario, shape: null })}>
          Random
        </Chip>
        {shapes.map((shape) => (
          <Chip key={shape} on={scenario.shape === shape} onClick={() => onChange({ ...scenario, shape })}>
            {shapeNames[shape]}
          </Chip>
        ))}
      </div>

      <div className={styles.row}>
        <span className={styles.label}>Mutators</span>
        <Chip on={scenario.mutators === null} onClick={() => onChange({ ...scenario, mutators: null })}>
          Random
        </Chip>
        <Chip on={scenario.mutators?.length === 0} onClick={() => onChange({ ...scenario, mutators: [] })}>
          None
        </Chip>
        {mutators.map((mutator) => (
          <Chip key={mutator} on={scenario.mutators?.includes(mutator) ?? false} onClick={() => toggleMutator(mutator)}>
            {mutatorInfo[mutator].name}
          </Chip>
        ))}
      </div>

      <div className={styles.row}>
        <span className={styles.label}>Add NPC</span>
        <Chip on={scenario.npcs === null} onClick={() => onChange({ ...scenario, npcs: null })}>
          Random
        </Chip>
        <Chip on={scenario.npcs?.length === 0} onClick={() => onChange({ ...scenario, npcs: [] })}>
          None
        </Chip>
        {npcStyles.map((style) => (
          <Chip
            key={style}
            on={false}
            onClick={() => npcs.length < MAX_NPCS && onChange({ ...scenario, npcs: [...npcs, style] })}
          >
            +{style}
          </Chip>
        ))}
      </div>
      {npcs.length > 0 ? (
        <div className={styles.row}>
          <span className={styles.label}>NPCs</span>
          {npcs.map((style, index) => (
            <Chip
              key={index}
              on
              onClick={() => onChange({ ...scenario, npcs: npcs.filter((_, other) => other !== index) })}
            >
              {style} ×
            </Chip>
          ))}
        </div>
      ) : null}
    </div>
  )
}

function Chip({ on, onClick, children }: { on: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button type="button" className={on ? styles.chipOn : styles.chip} onClick={onClick}>
      {children}
    </button>
  )
}
