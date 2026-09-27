import { type ReactNode, useState } from 'react'

import type { Mutator, NpcStyle, Scenario, Shape } from '@/types'
import { mutatorInfo, mutatorsClash, shapeNames } from '@/utils/round'

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
  // The button that starts the round, shown beside the close button.
  start: ReactNode
}

type Section = 'shape' | 'mutators' | 'npcs'

// The lobby card in its test form, which sets up the next round: its shape,
// its mutators and its NPCs' styles, one tab each, with what is picked in
// each shown on its tab. Each tab starts with a Random chip: while it is on,
// that part of the round is rolled as usual; turned off, the round gets
// exactly what is picked, which may be nothing. Every tab is the same grid
// of eight cells, with a slot for each NPC the round can have, the first
// empty one a menu to add one, so nothing moves as things are picked. It
// takes whatever room the arena has below the winner. The arena opens it
// from a hidden gesture (see Arena).
export default function ScenarioPanel({ scenario, onChange, onClose, start }: ScenarioPanelProps) {
  const [section, setSection] = useState<Section>('shape')
  const picked = scenario.mutators ?? []
  const npcs = scenario.npcs ?? []
  // Picking a mutator drops any picked that clash with it.
  const toggleMutator = (mutator: Mutator) =>
    onChange({
      ...scenario,
      mutators: picked.includes(mutator)
        ? picked.filter((other) => other !== mutator)
        : [...picked.filter((other) => !mutatorsClash(other, mutator)), mutator],
    })

  return (
    <div className={styles.panel}>
      <div className={styles.heading}>
        <span className={styles.title}>Test scenario</span>
        {start}
        <button type="button" className={styles.close} onClick={onClose} aria-label="Close test scenario">
          ×
        </button>
      </div>

      <div className={styles.tabs} role="tablist">
        <Tab on={section === 'shape'} onClick={() => setSection('shape')} name="Shape">
          {scenario.shape === null ? 'Random' : shapeNames[scenario.shape]}
        </Tab>
        <Tab on={section === 'mutators'} onClick={() => setSection('mutators')} name="Mutators">
          {scenario.mutators === null ? 'Random' : picked.length === 0 ? 'None' : picked.length}
        </Tab>
        <Tab on={section === 'npcs'} onClick={() => setSection('npcs')} name="NPCs">
          {scenario.npcs === null ? 'Random' : npcs.length === 0 ? 'None' : npcs.length}
        </Tab>
      </div>

      <div className={styles.grid} role="tabpanel">
        {section === 'shape' ? (
          <>
            <Chip random on={scenario.shape === null} onClick={() => onChange({ ...scenario, shape: null })}>
              Random
            </Chip>
            {shapes.map((shape) => (
              <Chip key={shape} on={scenario.shape === shape} onClick={() => onChange({ ...scenario, shape })}>
                {shapeNames[shape]}
              </Chip>
            ))}
          </>
        ) : section === 'mutators' ? (
          <>
            <Chip
              random
              on={scenario.mutators === null}
              onClick={() => onChange({ ...scenario, mutators: scenario.mutators === null ? [] : null })}
            >
              Random
            </Chip>
            {mutators.map((mutator) => (
              <Chip key={mutator} on={picked.includes(mutator)} onClick={() => toggleMutator(mutator)}>
                {mutatorInfo[mutator].name}
              </Chip>
            ))}
          </>
        ) : (
          <>
            <Chip
              random
              on={scenario.npcs === null}
              onClick={() => onChange({ ...scenario, npcs: scenario.npcs === null ? [] : null })}
            >
              Random
            </Chip>
            {Array.from({ length: MAX_NPCS }, (_, index) => {
              if (index < npcs.length)
                return (
                  <Chip
                    key={index}
                    on
                    title="Remove"
                    onClick={() => onChange({ ...scenario, npcs: npcs.filter((_, other) => other !== index) })}
                  >
                    {npcs[index]}
                    <span className={styles.remove}>×</span>
                  </Chip>
                )
              if (index === npcs.length)
                return (
                  <select
                    key={index}
                    className={styles.add}
                    value=""
                    aria-label="Add an NPC"
                    onChange={(event) => onChange({ ...scenario, npcs: [...npcs, event.target.value as NpcStyle] })}
                  >
                    <option value="" disabled>
                      + Add
                    </option>
                    {npcStyles.map((style) => (
                      <option key={style} value={style}>
                        {style}
                      </option>
                    ))}
                  </select>
                )
              return (
                <span key={index} className={styles.slot}>
                  {scenario.npcs === null ? '?' : ''}
                </span>
              )
            })}
          </>
        )}
      </div>
    </div>
  )
}

// A tab, named `name`, showing what is picked in its section.
function Tab({ on, onClick, name, children }: { on: boolean; onClick: () => void; name: string; children: ReactNode }) {
  return (
    <button type="button" role="tab" aria-selected={on} className={styles.tab} onClick={onClick}>
      <span className={styles.tabName}>{name}</span>
      <span className={styles.tabValue}>{children}</span>
    </button>
  )
}

// A button that is lit while `on`, or an action if that is left out.
type ChipProps = {
  on?: boolean
  onClick: () => void
  children: ReactNode
  random?: boolean
  title?: string
}

function Chip({ on, onClick, children, random, title }: ChipProps) {
  return (
    <button
      type="button"
      className={random ? styles.random : styles.chip}
      aria-pressed={on}
      title={title}
      onClick={onClick}
    >
      {children}
    </button>
  )
}
