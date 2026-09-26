// The current time on the server's clock, in Unix milliseconds. Round start
// times come from the server, so every comparison of one against the current
// time goes through here. serverTimeOffset is the server's clock minus the
// client's, measured when the server last reported its time; the message's
// travel time makes the server's clock read that much behind.
export function serverNow(serverTimeOffset: number): number {
  return Date.now() + serverTimeOffset
}
