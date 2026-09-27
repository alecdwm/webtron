import type { Scenario } from '@/types'

const SCENARIO_STORAGE_KEY = 'webtron:scenario'

// The test scenario last set up in this browser, or null when the scenario
// panel is closed.
export function readScenario(): Scenario | null {
  try {
    const stored = window.localStorage.getItem(SCENARIO_STORAGE_KEY)
    return stored ? (JSON.parse(stored) as Scenario) : null
  } catch {
    // Storage may be unavailable (private mode), or hold something else.
    return null
  }
}

export function storeScenario(scenario: Scenario | null) {
  try {
    if (scenario) window.localStorage.setItem(SCENARIO_STORAGE_KEY, JSON.stringify(scenario))
    else window.localStorage.removeItem(SCENARIO_STORAGE_KEY)
  } catch {
    // Storage may be unavailable (private mode); the scenario just won't persist.
  }
}
