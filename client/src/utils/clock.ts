// How long a sample of the server's clock counts towards the estimate of its
// offset. A sample older than this is dropped, so the estimate follows a
// connection whose latency rises, and recovers from a change to the server's
// clock.
const SAMPLE_WINDOW_MS = 10_000

// Samples of the server's clock minus the client's, each taken when a stamped
// message arrived. Each one reads behind by that message's travel time, so
// the largest recent one is the closest.
let samples: { takenAt: number; offset: number }[] = []

// The current time on the server's clock, in Unix milliseconds. Round start
// times and update stamps come from the server, so every comparison of one
// against the current time goes through here. serverTimeOffset is the
// server's clock minus the client's, from sampleServerClock. The client's
// clock is performance.now(), which a change to the system clock doesn't
// move.
export function serverNow(serverTimeOffset: number): number {
  return performance.now() + serverTimeOffset
}

// Adds a sample of the server's clock, the stamp of a message that has just
// arrived, and returns the new estimate of serverTimeOffset.
export function sampleServerClock(serverTime: number): number {
  const now = performance.now()
  samples = samples.filter(({ takenAt }) => now - takenAt <= SAMPLE_WINDOW_MS)
  samples.push({ takenAt: now, offset: serverTime - now })
  return samples.reduce((best, { offset }) => Math.max(best, offset), -Infinity)
}

// Forgets every sample, for a connection to a server whose clock may differ.
export function resetServerClock() {
  samples = []
}
