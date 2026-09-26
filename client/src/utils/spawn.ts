// Lightcycles rez in one after another at the start of a round, in the order
// the arena lists them. Returns how long the one at index waits, capped so a
// full arena is done well before the countdown ends.
export function spawnDelayMs(index: number) {
  return Math.min(index, 8) * 70
}
