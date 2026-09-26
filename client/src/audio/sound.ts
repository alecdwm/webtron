// Synthesised sound effects built on the Web Audio API. There are no audio
// assets: every sound is assembled from oscillators and filtered noise.
//
// Browsers only allow an AudioContext to start after a user gesture, so the
// context is created in unlockAudio(), which App calls on user input events.
// Until then every play function is a silent no-op. iOS Safari only counts
// some events as gestures (touchend and click, but not a touch pointerdown),
// so App listens for all of them.

const MUTED_STORAGE_KEY = 'webtron:muted'
const MASTER_VOLUME = 0.6

// Lightcycle speeds from src/server/arena.rs. Engines pitch up with speed
// between these two while slipstreaming.
const LIGHTCYCLE_SPEED = 55
const LIGHTCYCLE_BOOST_SPEED = 110
const ENGINE_BOOST_PITCH = 1.5

let ctx: AudioContext | null = null
let master: GainNode | null = null
let noiseBuffer: AudioBuffer | null = null

let muted = readMuted()
const mutedListeners = new Set<() => void>()

function readMuted() {
  try {
    return window.localStorage.getItem(MUTED_STORAGE_KEY) === '1'
  } catch {
    return false
  }
}

export function unlockAudio() {
  if (ctx === null) {
    // Play through the iOS ring/silent switch, like media playback does,
    // instead of being muted by it (Safari 16.4+).
    const audioSession = (navigator as unknown as { audioSession?: { type: string } }).audioSession
    if (audioSession) audioSession.type = 'playback'

    const AudioContextClass =
      window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
    if (!AudioContextClass) return
    ctx = new AudioContextClass()
    master = ctx.createGain()
    master.gain.value = muted ? 0 : MASTER_VOLUME
    master.connect(ctx.destination)
  }
  // iOS also reports 'interrupted', e.g. after a phone call.
  if (ctx.state !== 'running' && ctx.state !== 'closed' && document.visibilityState === 'visible') {
    void ctx.resume()
    // Older iOS versions only unlock output once a sound starts inside the
    // gesture, so play a single silent sample.
    const silence = ctx.createBufferSource()
    silence.buffer = ctx.createBuffer(1, 1, ctx.sampleRate)
    silence.connect(ctx.destination)
    silence.start()
  }
}

// Silence everything while the tab is hidden.
document.addEventListener('visibilitychange', () => {
  if (ctx === null) return
  if (document.visibilityState === 'hidden') void ctx.suspend()
  else void ctx.resume()
})

export function isMuted() {
  return muted
}
export function setMuted(value: boolean) {
  muted = value
  try {
    window.localStorage.setItem(MUTED_STORAGE_KEY, value ? '1' : '0')
  } catch {
    // Storage may be unavailable (private mode); the setting just won't persist.
  }
  if (ctx && master) master.gain.setTargetAtTime(value ? 0 : MASTER_VOLUME, ctx.currentTime, 0.02)
  mutedListeners.forEach((listener) => listener())
}
export function subscribeMuted(listener: () => void) {
  mutedListeners.add(listener)
  return () => mutedListeners.delete(listener)
}

function audio() {
  if (ctx === null || master === null) return null
  // A context created during the unlocking gesture may still be resuming;
  // sounds scheduled on it play as soon as it starts. While the tab is hidden,
  // drop sounds instead so they don't all fire at once on return.
  if (ctx.state !== 'running' && !(ctx.state === 'suspended' && document.visibilityState === 'visible')) return null
  return { ctx, out: master }
}

function getNoiseBuffer(ctx: AudioContext) {
  if (noiseBuffer === null) {
    noiseBuffer = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate)
    const data = noiseBuffer.getChannelData(0)
    for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1
  }
  return noiseBuffer
}

// A smooth random line between -1 and 1, looping every 30 seconds: a random
// point every 0.3 seconds, joined by cosine interpolation.
let wanderBuffer: AudioBuffer | null = null
function getWanderBuffer(ctx: AudioContext) {
  if (wanderBuffer === null) {
    const rate = 8000
    const seconds = 30
    const points = Array.from({ length: 100 }, () => Math.random() * 2 - 1)
    const perPoint = (rate * seconds) / points.length
    wanderBuffer = ctx.createBuffer(1, rate * seconds, rate)
    const data = wanderBuffer.getChannelData(0)
    for (let i = 0; i < data.length; i++) {
      const k = Math.floor(i / perPoint)
      const f = i / perPoint - k
      const a = points[k]
      const b = points[(k + 1) % points.length]
      data[i] = a + ((b - a) * (1 - Math.cos(Math.PI * f))) / 2
    }
  }
  return wanderBuffer
}

// Connects a source through an optional stereo panner to the master bus.
function output(ctx: AudioContext, out: AudioNode, pan = 0) {
  if (pan === 0 || !ctx.createStereoPanner) return out
  const panner = ctx.createStereoPanner()
  panner.pan.value = Math.max(-1, Math.min(1, pan))
  panner.connect(out)
  return panner
}

type ToneOptions = {
  type?: OscillatorType
  freq: number
  freqEnd?: number
  duration: number
  gain?: number
  attack?: number
  delay?: number
  pan?: number
  filter?: { type: BiquadFilterType; freq: number; freqEnd?: number; q?: number }
}

function tone({
  type = 'square',
  freq,
  freqEnd,
  duration,
  gain = 0.1,
  attack = 0.005,
  delay = 0,
  pan = 0,
  filter,
}: ToneOptions) {
  const a = audio()
  if (a === null) return
  const { ctx, out } = a
  const t = ctx.currentTime + delay

  const osc = ctx.createOscillator()
  osc.type = type
  osc.frequency.setValueAtTime(freq, t)
  if (freqEnd !== undefined) osc.frequency.exponentialRampToValueAtTime(Math.max(1, freqEnd), t + duration)

  const env = ctx.createGain()
  env.gain.setValueAtTime(0, t)
  env.gain.linearRampToValueAtTime(gain, t + attack)
  env.gain.exponentialRampToValueAtTime(0.0001, t + duration)

  let node: AudioNode = osc
  if (filter) {
    const biquad = ctx.createBiquadFilter()
    biquad.type = filter.type
    biquad.Q.value = filter.q ?? 1
    biquad.frequency.setValueAtTime(filter.freq, t)
    if (filter.freqEnd !== undefined) biquad.frequency.exponentialRampToValueAtTime(filter.freqEnd, t + duration)
    node.connect(biquad)
    node = biquad
  }
  node.connect(env)
  env.connect(output(ctx, out, pan))

  osc.start(t)
  osc.stop(t + duration + 0.05)
}

type NoiseOptions = {
  duration: number
  gain?: number
  delay?: number
  pan?: number
  attack?: number
  filter: { type: BiquadFilterType; freq: number; freqEnd?: number; q?: number }
}

function noise({ duration, gain = 0.1, delay = 0, pan = 0, attack = 0.005, filter }: NoiseOptions) {
  const a = audio()
  if (a === null) return
  const { ctx, out } = a
  const t = ctx.currentTime + delay

  const source = ctx.createBufferSource()
  source.buffer = getNoiseBuffer(ctx)
  source.loop = true

  const biquad = ctx.createBiquadFilter()
  biquad.type = filter.type
  biquad.Q.value = filter.q ?? 1
  biquad.frequency.setValueAtTime(filter.freq, t)
  if (filter.freqEnd !== undefined) biquad.frequency.exponentialRampToValueAtTime(filter.freqEnd, t + duration)

  const env = ctx.createGain()
  env.gain.setValueAtTime(0, t)
  env.gain.linearRampToValueAtTime(gain, t + attack)
  env.gain.exponentialRampToValueAtTime(0.0001, t + duration)

  source.connect(biquad)
  biquad.connect(env)
  env.connect(output(ctx, out, pan))

  source.start(t, Math.random() * 0.5)
  source.stop(t + duration + 0.05)
}

// Menu sounds

export function playHover() {
  tone({ type: 'sine', freq: 1400, freqEnd: 1800, duration: 0.05, gain: 0.03 })
}

export function playClick() {
  tone({
    type: 'square',
    freq: 520,
    freqEnd: 1040,
    duration: 0.09,
    gain: 0.05,
    filter: { type: 'lowpass', freq: 3000 },
  })
  tone({ type: 'sine', freq: 1560, duration: 0.12, gain: 0.04, delay: 0.04 })
}

// Pentatonic step per colour so each choice has its own note.
const SELECT_NOTES = [440, 494, 554, 659, 740, 880]
export function playSelect(index = 0) {
  const freq = SELECT_NOTES[index % SELECT_NOTES.length]
  tone({ type: 'triangle', freq, duration: 0.14, gain: 0.08 })
  tone({ type: 'square', freq: freq * 2, duration: 0.06, gain: 0.02, filter: { type: 'lowpass', freq: 2500 } })
}

export function playType() {
  noise({ duration: 0.03, gain: 0.05, filter: { type: 'highpass', freq: 4000 } })
  tone({ type: 'square', freq: 2200 + Math.random() * 400, duration: 0.02, gain: 0.015 })
}

export function playConnect() {
  tone({
    type: 'sawtooth',
    freq: 110,
    freqEnd: 880,
    duration: 0.6,
    gain: 0.07,
    attack: 0.05,
    filter: { type: 'lowpass', freq: 400, freqEnd: 5000, q: 6 },
  })
  tone({ type: 'sine', freq: 1760, duration: 0.3, gain: 0.04, delay: 0.45 })
}

export function playWhoosh() {
  noise({ duration: 0.4, gain: 0.07, attack: 0.12, filter: { type: 'bandpass', freq: 300, freqEnd: 3500, q: 1.5 } })
}

export function playError() {
  tone({ type: 'square', freq: 180, duration: 0.18, gain: 0.06, filter: { type: 'lowpass', freq: 1200 } })
  tone({ type: 'square', freq: 120, duration: 0.3, gain: 0.06, delay: 0.2, filter: { type: 'lowpass', freq: 1200 } })
}

// Arena sounds

export function playPlayerJoin() {
  tone({ type: 'triangle', freq: 660, duration: 0.1, gain: 0.06 })
  tone({ type: 'triangle', freq: 990, duration: 0.16, gain: 0.06, delay: 0.08 })
}

export function playPlayerLeave() {
  tone({ type: 'triangle', freq: 660, duration: 0.1, gain: 0.05 })
  tone({ type: 'triangle', freq: 440, duration: 0.16, gain: 0.05, delay: 0.08 })
}

export function playCountdownBeep() {
  tone({ type: 'square', freq: 440, duration: 0.22, gain: 0.07, filter: { type: 'lowpass', freq: 2400 } })
  tone({ type: 'sine', freq: 880, duration: 0.22, gain: 0.03 })
}

export function playCountdownGo() {
  tone({ type: 'square', freq: 880, duration: 0.6, gain: 0.07, filter: { type: 'lowpass', freq: 3200 } })
  tone({ type: 'sawtooth', freq: 440, duration: 0.6, gain: 0.04, filter: { type: 'lowpass', freq: 1800 } })
  tone({ type: 'sine', freq: 1760, duration: 0.4, gain: 0.03 })
}

// A lightcycle rezzing in: a rising sweep with a trail of climbing blips, the
// reverse of the derez. delay staggers the lightcycles of one round.
export function playSpawn(self: boolean, pan = 0, delay = 0) {
  const gain = self ? 1 : 0.45
  noise({
    duration: 0.45,
    gain: 0.05 * gain,
    attack: 0.3,
    delay,
    pan,
    filter: { type: 'bandpass', freq: 600, freqEnd: 7000, q: 4 },
  })
  tone({
    type: 'sawtooth',
    freq: 110,
    freqEnd: 660,
    duration: 0.45,
    gain: 0.05 * gain,
    attack: 0.3,
    delay,
    pan,
    filter: { type: 'lowpass', freq: 500, freqEnd: 4000, q: 5 },
  })
  for (let i = 0; i < 5; i++) {
    const freq = 900 + i * 260 + Math.random() * 120
    tone({
      type: 'square',
      freq,
      freqEnd: freq * 1.5,
      duration: 0.04,
      gain: 0.02 * gain,
      pan,
      delay: delay + 0.1 + i * 0.06,
      filter: { type: 'lowpass', freq: 4000 },
    })
  }
  tone({ type: 'sine', freq: 1320, duration: 0.35, gain: 0.04 * gain, delay: delay + 0.42, pan })
}

export function playTurn(self: boolean, pan = 0) {
  const gain = self ? 0.1 : 0.035
  tone({
    type: 'sawtooth',
    freq: 260,
    freqEnd: 820,
    duration: 0.08,
    gain,
    pan,
    filter: { type: 'bandpass', freq: 900, q: 3 },
  })
  noise({ duration: 0.04, gain: gain * 0.6, pan, filter: { type: 'highpass', freq: 5000 } })
}

export function playCrash(self: boolean, pan = 0) {
  const gain = self ? 1 : 0.55
  noise({ duration: 0.9, gain: 0.3 * gain, pan, filter: { type: 'lowpass', freq: 6000, freqEnd: 120 } })
  tone({ type: 'square', freq: 240, freqEnd: 40, duration: 0.7, gain: 0.08 * gain, pan })
  tone({ type: 'sine', freq: 90, freqEnd: 28, duration: 0.6, gain: 0.25 * gain, pan })
  // Derez crackle: a scatter of falling digital blips.
  for (let i = 0; i < 7; i++) {
    const freq = 1800 - i * 200 + Math.random() * 300
    tone({ type: 'square', freq, freqEnd: freq * 0.6, duration: 0.05, gain: 0.03 * gain, pan, delay: 0.06 + i * 0.05 })
  }
}

export function playDerez(pan = 0) {
  noise({ duration: 0.9, gain: 0.05, attack: 0.05, pan, filter: { type: 'bandpass', freq: 5000, freqEnd: 400, q: 4 } })
  tone({ type: 'sine', freq: 660, freqEnd: 110, duration: 0.9, gain: 0.04, attack: 0.02, pan })
  // Glittering blips falling away as the trail breaks up.
  for (let i = 0; i < 10; i++) {
    const freq = 2400 - i * 180 + Math.random() * 200
    tone({
      type: 'square',
      freq,
      freqEnd: freq * 0.5,
      duration: 0.04,
      gain: 0.018,
      pan,
      delay: i * 0.07,
      filter: { type: 'lowpass', freq: 4000 },
    })
  }
}

export function playWin() {
  const notes = [523, 659, 784, 1047]
  notes.forEach((freq, i) => {
    tone({ type: 'square', freq, duration: 0.22, gain: 0.05, delay: i * 0.11, filter: { type: 'lowpass', freq: 3500 } })
    tone({ type: 'triangle', freq: freq / 2, duration: 0.22, gain: 0.05, delay: i * 0.11 })
  })
  tone({ type: 'sine', freq: 2094, duration: 0.8, gain: 0.03, delay: 0.44 })
}

export function playLose() {
  const notes = [392, 330, 262]
  notes.forEach((freq, i) => {
    tone({ type: 'square', freq, duration: 0.28, gain: 0.05, delay: i * 0.16, filter: { type: 'lowpass', freq: 1800 } })
  })
}

// Engines

// The player's own engine sits on the root pitch. Every other engine takes a
// slot on a pentatonic scale above and below it, so a full arena forms a chord
// rather than a cluster of near-unisons beating against each other.
const ENGINE_ROOT_FREQ = 64
const ENGINE_SLOT_RATIOS = [3 / 2, 5 / 4, 5 / 3, 2, 9 / 8, 3 / 4, 9 / 4]
// Gain for each slot that evens out its loudness. Lower slots lose more to the
// crowd bus highpass and to the ear's weaker response to low frequencies.
const ENGINE_SLOT_GAINS = [1.22, 1.39, 1.19, 1.08, 1.38, 1.78, 1]
const takenSlots = new Set<number>()

// Other players' engines mix through one bus. Its highpass leaves the low end
// to the player's own engine, and its gain caps the crowd's summed power at
// CROWD_MAX_POWER times that of one engine right beside the player. One close
// engine plays at full level against the player's own; several close ones are
// turned down together so the player's own engine is still heard among them.
const CROWD_HIGHPASS = 90
const CROWD_MAX_POWER = 1.5
let crowd: { bus: GainNode; engines: Set<Engine> } | null = null

function crowdBus(ctx: AudioContext, out: AudioNode) {
  if (crowd === null) {
    const bus = ctx.createGain()
    const highpass = ctx.createBiquadFilter()
    highpass.type = 'highpass'
    highpass.frequency.value = CROWD_HIGHPASS
    highpass.Q.value = 0.7
    bus.connect(highpass)
    highpass.connect(out)
    crowd = { bus, engines: new Set() }
  }
  return crowd
}

function updateCrowdGain(ctx: AudioContext) {
  if (crowd === null) return
  let power = 0
  crowd.engines.forEach((engine) => (power += engine.power()))
  crowd.bus.gain.setTargetAtTime(Math.min(1, Math.sqrt(CROWD_MAX_POWER / Math.max(power, 1e-6))), ctx.currentTime, 0.1)
}

// A deterministic random sequence for a string, so a lightcycle keeps the same
// engine character from round to round.
function seededRandom(key: string) {
  let a = 2166136261
  for (let i = 0; i < key.length; i++) a = Math.imul(a ^ key.charCodeAt(i), 16777619)
  return () => {
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

// Frequency ratio of each oscillator to the engine's pitch: two saws and a sub
// square an octave down, then on the player's own engine only, a saw an octave
// up and a flutter two octaves down that pulses the engine's level.
const ENGINE_OSC_RATIOS = [1, 1, 0.5, 2, 0.25]

// How far the flutter pulls the player's own engine's level down.
const ENGINE_FLUTTER_DEPTH = 0.18
// Level of the rasp on the player's own engine. At 1.5 it measures about 16 dB
// under the engine's tone, K-weighted.
const ENGINE_RASP_LEVEL = 1.5

// Engine levels, set by measuring K-weighted loudness at the master output:
// the player's own engine alone sits a little above one other engine right
// beside it.
const ENGINE_SELF_LEVEL = 0.08
const ENGINE_OTHER_LEVEL = 0.04

// Engines start up just before the round, timed back from its start so that
// each catches a moment (its plan's catchLead) before it and is still
// settling as it launches, revving up to cruising over ENGINE_LAUNCH seconds.
// ENGINE_REV_UP is the longest a startup takes. The revs are simulated.
// Starting up, the starter cranks the engine at ENGINE_CRANK_RPM,
// straining and slowing with each compression. The engine is silent until it
// catches, apart from the starter and a sputter of single combustion pops.
// Then it fades in over ENGINE_CATCH_FADE seconds as the throttle opens for
// ENGINE_CATCH_SECONDS, the revs flare, and they hunt around idle as they
// settle. After the catch the throttle drives the load at once and
// the revs with inertia, quicker to rise than to fall. The launch opens with a
// surge of load up to ENGINE_LAUNCH_SURGE that eases back to cruising as the
// revs climb, making the engine louder and snarling through a mid boost of up
// to ENGINE_SNARL_DB at ENGINE_SNARL_FREQ.
//
// An engine started with little time to go cranks for less time, or catches
// at once, and one started after the round has begun just revs up from idle
// over ENGINE_LATE_LAUNCH seconds.
//
// Revs are relative to cruising, where the engine plays at its set pitch, and
// load runs from 0 (throttle closed) to 1 (open).
export const ENGINE_REV_UP = 1.4
const ENGINE_LAUNCH = 0.9
const ENGINE_LAUNCH_SURGE = 2.2
const ENGINE_SNARL_DB = 10
const ENGINE_SNARL_FREQ = 800
const ENGINE_LATE_LAUNCH = 0.3
const ENGINE_IDLE_RPM = 0.5
const ENGINE_CRANK_RPM = 0.18
const ENGINE_CATCH_SECONDS = 0.12
const ENGINE_CATCH_FADE = 0.1
const ENGINE_CATCH_RISE = 0.06
// Time constants of the revs rising and falling towards the throttle's.
const ENGINE_RPM_RISE = 0.15
const ENGINE_RPM_FALL = 0.25
const ENGINE_LOAD_SMOOTHING = 0.015
const REV_UP_STEPS_PER_SECOND = 500
// The starter motor's pitch while cranking freely, and its level and that of
// the sputter's pops relative to the engine's.
const STARTER_FREQ = 160
const STARTER_LEVEL = 0.6
const POP_FREQ = 80
const POP_LEVEL = 1.2

// The filter wobble, and on the player's own engine the flutter, give an
// idling engine its putter. As the revs rise to cruising they fade to this
// fraction, leaving a smoother running sound.
const ENGINE_CRUISE_PUTTER = 0.3

// A running engine's revs are never quite steady: each wanders along its own
// smooth random line, by up to ENGINE_WANDER_CENTS in pitch, with its snarl
// and level moving with it as though the throttle were shifting.
const ENGINE_WANDER_CENTS = 30
const ENGINE_WANDER_SNARL_DB = 3
const ENGINE_WANDER_LEVEL = 0.08
// Each turn snarls the engine through this much mid boost, easing off after.
const ENGINE_TURN_SNARL_DB = 8

// How each engine starts up, so engines don't all start in unison.
type StartupPlan = {
  crank: number
  // When the sputter's pops come, in seconds relative to the catch.
  pops: number[]
  // How long before the round starts the engine catches.
  catchLead: number
  // Compression strokes a second while cranking.
  crankRate: number
  flareRpm: number
  huntRate: number
}

function planStartup(random: () => number): StartupPlan {
  const between = (min: number, max: number) => min + random() * (max - min)
  const pops = []
  for (let at = -between(0.15, 0.3); at < 0.02; at += between(0.03, 0.09)) pops.push(at)
  return {
    crank: between(0.35, 0.8),
    pops,
    catchLead: between(0.3, 0.5),
    crankRate: between(4.5, 6.5),
    flareRpm: between(0.8, 1),
    huntRate: between(2, 3.5),
  }
}

// Simulates the revs and load at each step from an engine starting up
// untilStart seconds before the round, to cruising after its launch. The last
// step is exactly cruising: full revs, open throttle. The starter turns from
// crankStart until the engine catches at crankEnd, and firing runs from 0
// before the catch to 1 once the engine runs.
function simulateRevUp(untilStart: number, plan: StartupPlan) {
  const idle = Math.max(0, untilStart)
  const launch = untilStart > 0 ? ENGINE_LAUNCH : ENGINE_LATE_LAUNCH
  const duration = idle + launch
  const steps = Math.ceil(duration * REV_UP_STEPS_PER_SECOND)
  const dt = duration / (steps - 1)
  const crankEnd = idle - plan.catchLead >= 0.1 ? idle - plan.catchLead : 0
  const crankStart = Math.max(0, crankEnd - plan.crank)
  const catches = untilStart > 0
  const rpm = new Float32Array(steps)
  const load = new Float32Array(steps)
  const firing = new Float32Array(steps)
  let r = catches && crankEnd > 0 ? ENGINE_CRANK_RPM : ENGINE_IDLE_RPM
  let l = 0
  let launchFromRpm = r
  for (let i = 0; i < steps; i++) {
    const t = i * dt
    let throttle = 1
    if (t < crankStart) {
      throttle = 0
      launchFromRpm = r
    } else if (t < crankEnd) {
      // Each compression slows the starter, and the revs pick up past it.
      const phase = ((t - crankStart) * plan.crankRate) % 1
      const compression = Math.sin(Math.PI * phase) ** 4
      r = ENGINE_CRANK_RPM * (1.15 - 0.4 * compression)
      throttle = 0.4 * (1 - compression)
      launchFromRpm = r
    } else if (t < idle) {
      const sinceCatch = t - crankEnd
      throttle = catches && sinceCatch < ENGINE_CATCH_SECONDS ? 1 : 0
      const hunt = 0.08 * Math.exp(-sinceCatch / 0.5) * Math.sin(2 * Math.PI * plan.huntRate * sinceCatch)
      const target = throttle ? plan.flareRpm : ENGINE_IDLE_RPM * (1 + hunt)
      const tau = target > r ? (throttle ? ENGINE_CATCH_RISE : ENGINE_RPM_RISE) : ENGINE_RPM_FALL
      r += (target - r) * Math.min(1, dt / tau)
      launchFromRpm = r
    } else {
      // The revs climb quickly as the lightcycle pulls away and ease into
      // cruising, while the surge eases off.
      const x = (t - idle) / launch
      r = launchFromRpm + (1 - launchFromRpm) * (1 - (1 - x) ** 2)
      if (untilStart > 0) throttle = 1 + (ENGINE_LAUNCH_SURGE - 1) * (1 - x) ** 1.5
    }
    l += (throttle - l) * Math.min(1, dt / ENGINE_LOAD_SMOOTHING)
    rpm[i] = r
    load[i] = l
    firing[i] = crankEnd > 0 ? Math.max(0, Math.min(1, (t - crankEnd) / ENGINE_CATCH_FADE)) : 1
  }
  rpm[steps - 1] = 1
  load[steps - 1] = 1
  return { rpm, load, firing, duration, crankStart, crankEnd }
}

// Continuous engine drone for one lightcycle: two detuned saws and a sub square
// through a lowpass filter whose cutoff is wobbled by an LFO. A slow second LFO
// drifts the tuning by a few cents so the drone doesn't sit dead still.
//
// Each engine's tuning, filter and wobble vary within a range, seeded by its
// key. Engines far from the player's lightcycle (see setProximity) are quieter
// and darker, so the nearest few stand out of a crowd.
//
// The player's own engine has a smaller sub than the others, an octave-up saw
// and a shallower wobble. For texture it has a chugging flutter and a rasp:
// noise gated by its own saw, so the grit is pitched with the engine and sits
// under the same filter as the rest of its tone.
export class Engine {
  private nodes: { wobble: OscillatorNode; stop: (t: number) => void } | null = null
  private panner: StereoPannerNode | null = null
  private oscs: OscillatorNode[] = []
  private env: GainNode | null = null
  private level: GainNode | null = null
  private filter: BiquadFilterNode | null = null
  private slot: number | null = null
  private baseFreq = ENGINE_ROOT_FREQ
  private pitch = 1
  private proximity = 1
  private doppler = 1
  private snarl: BiquadFilterNode | null = null
  private revUpEnds = 0
  private readonly voice: {
    slotPreference: number
    cents: number
    spread: number
    cutoff: number
    q: number
    wobbleRate: number
    wobbleDepth: number
    driftRate: number
    driftDepth: number
    sub: number
    startup: StartupPlan
  }

  constructor(
    private self: boolean,
    key: string,
  ) {
    const random = seededRandom(key)
    const between = (min: number, max: number) => min + random() * (max - min)
    this.voice = {
      slotPreference: Math.floor(random() * ENGINE_SLOT_RATIOS.length),
      cents: self ? 0 : between(-8, 8),
      spread: between(5, 13),
      cutoff: (self ? 750 : 400) * between(0.85, 1.15),
      q: between(3, 5.5),
      wobbleRate: between(5, 8.5),
      wobbleDepth: between(60, 120) * (self ? 0.6 : 1),
      driftRate: between(0.08, 0.3),
      driftDepth: between(3, 9),
      sub: self ? 0.35 : between(0.4, 0.8),
      startup: planStartup(random),
    }
  }

  // Starts the engine untilStart seconds before the round starts, or after it
  // if negative.
  start(pan = 0, untilStart = ENGINE_REV_UP) {
    const a = audio()
    if (a === null || this.nodes !== null) return
    const { ctx, out } = a
    const t = ctx.currentTime
    const voice = this.voice
    const sim = simulateRevUp(untilStart, voice.startup)
    // The rev-up is automated on parameters of its own (the rev filter, snarl
    // and rev gain, the wobble and flutter depths and the oscillators' detune)
    // so it can't collide with the automation that setProximity, setSpeed,
    // setDoppler and stop schedule. rev waits for it to finish before touching
    // the snarl.
    const automate = (param: AudioParam, value: (rpm: number, load: number, firing: number) => number) => {
      const curve = new Float32Array(sim.rpm.length)
      for (let i = 0; i < curve.length; i++) curve[i] = value(sim.rpm[i], sim.load[i], sim.firing[i])
      param.setValueCurveAtTime(curve, t, sim.duration)
    }
    const putter = (rpm: number) =>
      1 - (1 - ENGINE_CRUISE_PUTTER) * Math.max(0, Math.min(1, (rpm - ENGINE_IDLE_RPM) / (1 - ENGINE_IDLE_RPM)))
    this.revUpEnds = t + sim.duration

    if (!this.self) {
      this.slot = takeSlot(voice.slotPreference)
      this.baseFreq = ENGINE_ROOT_FREQ * ENGINE_SLOT_RATIOS[this.slot]
    }

    const filter = ctx.createBiquadFilter()
    filter.type = 'lowpass'
    filter.frequency.value = this.cutoff()
    filter.Q.value = voice.q

    const highpass = ctx.createBiquadFilter()
    highpass.type = 'highpass'
    highpass.frequency.value = this.self ? 55 : 40
    highpass.Q.value = 0.7

    const wobble = ctx.createOscillator()
    wobble.frequency.setValueAtTime(voice.wobbleRate * 0.4, t)
    wobble.frequency.linearRampToValueAtTime(voice.wobbleRate, t + sim.duration)
    const wobbleDepth = ctx.createGain()
    automate(wobbleDepth.gain, (rpm) => voice.wobbleDepth * putter(rpm))
    wobble.connect(wobbleDepth)
    wobbleDepth.connect(filter.frequency)

    const drift = ctx.createOscillator()
    drift.frequency.value = voice.driftRate
    const driftDepth = ctx.createGain()
    driftDepth.gain.value = voice.driftDepth
    drift.connect(driftDepth)

    const wander = ctx.createBufferSource()
    wander.buffer = getWanderBuffer(ctx)
    wander.loop = true
    const wanderCents = ctx.createGain()
    wanderCents.gain.value = ENGINE_WANDER_CENTS
    wander.connect(wanderCents)

    const env = ctx.createGain()
    const envLevel = this.self ? ENGINE_SELF_LEVEL : ENGINE_OTHER_LEVEL * ENGINE_SLOT_GAINS[this.slot ?? 0]
    env.gain.setValueAtTime(0, t)
    env.gain.linearRampToValueAtTime(envLevel, t + 0.2)

    // Level follows the revs and the load, and brightness follows the load
    // more than the revs, so closing the throttle darkens and quietens the
    // engine at once while the revs are still falling.
    const revFilter = ctx.createBiquadFilter()
    revFilter.type = 'lowpass'
    revFilter.Q.value = 2
    automate(revFilter.frequency, (rpm, load) => {
      const revs = Math.max(0, Math.min(1, (rpm - ENGINE_IDLE_RPM) / (1 - ENGINE_IDLE_RPM)))
      return Math.min(20000, 200 * 100 ** (0.4 * revs + 0.6 * Math.min(1, load)))
    })
    const snarl = ctx.createBiquadFilter()
    snarl.type = 'peaking'
    snarl.frequency.value = ENGINE_SNARL_FREQ
    snarl.Q.value = 1.2
    automate(snarl.gain, (_rpm, load) => (ENGINE_SNARL_DB * Math.max(0, load - 1)) / (ENGINE_LAUNCH_SURGE - 1))
    const revGain = ctx.createGain()
    automate(revGain.gain, (rpm, load, firing) => rpm * (0.5 + 0.5 * load) * firing)
    const wanderSnarl = ctx.createGain()
    wanderSnarl.gain.value = ENGINE_WANDER_SNARL_DB
    wander.connect(wanderSnarl)
    wanderSnarl.connect(snarl.gain)
    const wanderLevel = ctx.createGain()
    wanderLevel.gain.value = ENGINE_WANDER_LEVEL
    wander.connect(wanderLevel)
    wanderLevel.connect(revGain.gain)

    const level = ctx.createGain()
    level.gain.value = this.levelGain()

    // The starter motor strains louder and slows through each compression, as
    // the revs do, and spins down once the engine catches. The sputter's pops
    // are single low barks, each dropping in pitch.
    let starter: OscillatorNode | null = null
    let popper: OscillatorNode | null = null
    if (sim.crankEnd > 0) {
      const dt = sim.duration / (sim.rpm.length - 1)
      const crankFrom = Math.ceil(sim.crankStart / dt)
      const crank = sim.rpm.slice(crankFrom, Math.floor(sim.crankEnd / dt))
      const crankAt = t + crankFrom * dt
      const crankLength = crank.length * dt
      starter = ctx.createOscillator()
      starter.type = 'sawtooth'
      starter.frequency.setValueCurveAtTime(
        crank.map((rpm) => (STARTER_FREQ * rpm) / ENGINE_CRANK_RPM),
        crankAt,
        crankLength,
      )
      starter.frequency.exponentialRampToValueAtTime(STARTER_FREQ * 0.3, t + sim.crankEnd + 0.25)
      const starterFilter = ctx.createBiquadFilter()
      starterFilter.type = 'lowpass'
      starterFilter.frequency.value = 1000
      const starterGain = ctx.createGain()
      const starterLevel = envLevel * STARTER_LEVEL
      starterGain.gain.setValueCurveAtTime(
        crank.map((rpm, i) => {
          const strain = (1.15 - rpm / ENGINE_CRANK_RPM) / 0.4
          return starterLevel * (0.55 + 0.45 * strain) * Math.min(1, (i * dt) / 0.03)
        }),
        crankAt,
        crankLength,
      )
      starterGain.gain.linearRampToValueAtTime(0, t + sim.crankEnd + 0.2)
      starter.connect(starterFilter)
      starterFilter.connect(starterGain)
      starterGain.connect(level)
      starter.start(crankAt)
      starter.stop(t + sim.crankEnd + 0.3)

      const pop = ctx.createOscillator()
      popper = pop
      pop.type = 'sawtooth'
      const popFilter = ctx.createBiquadFilter()
      popFilter.type = 'lowpass'
      popFilter.frequency.value = 600
      popFilter.Q.value = 2
      const popGain = ctx.createGain()
      popGain.gain.value = 0
      const pops = voice.startup.pops.map((offset) => t + Math.max(sim.crankStart + 0.05, sim.crankEnd + offset))
      pops.forEach((at, i) => {
        // Each pop ends before the next begins, so their automation doesn't
        // overlap.
        const length = Math.min(0.07, (pops[i + 1] ?? Infinity) - at - 0.005)
        const peak = envLevel * POP_LEVEL * (0.6 + 0.4 * Math.random())
        pop.frequency.setValueAtTime(POP_FREQ * 1.4, at)
        pop.frequency.exponentialRampToValueAtTime(POP_FREQ * 0.7, at + length)
        popGain.gain.setValueAtTime(0, at)
        popGain.gain.linearRampToValueAtTime(peak, at + 0.004)
        popGain.gain.exponentialRampToValueAtTime(0.0001, at + length)
        popGain.gain.setValueAtTime(0, at + length + 0.001)
      })
      pop.connect(popFilter)
      popFilter.connect(popGain)
      popGain.connect(level)
      pop.start(t)
      pop.stop(t + sim.crankEnd + 0.2)
    }

    const makeOsc = (type: OscillatorType, index: number, detune: number, gain = 1) => {
      const osc = ctx.createOscillator()
      osc.type = type
      osc.frequency.value = this.oscFreq(index)
      osc.detune.value = voice.cents + detune
      driftDepth.connect(osc.detune)
      wanderCents.connect(osc.detune)
      const oscGain = ctx.createGain()
      oscGain.gain.value = gain
      osc.connect(oscGain)
      oscGain.connect(filter)
      return osc
    }
    this.oscs = [
      makeOsc('sawtooth', 0, -voice.spread),
      makeOsc('sawtooth', 1, voice.spread),
      makeOsc('square', 2, 0, voice.sub),
    ]
    // The octave-up saw keeps the player's own engine, the lowest voice in the
    // arena, audible on small speakers that can't reproduce its fundamental.
    if (this.self) this.oscs.push(makeOsc('sawtooth', 3, 0, 0.6))

    filter.connect(highpass)
    let texture: { flutter: OscillatorNode; rasp: AudioBufferSourceNode } | null = null
    if (this.self) {
      // The flutter swings the grain gain between 1 - 2 * depth and 1.
      const grain = ctx.createGain()
      grain.gain.value = 1 - ENGINE_FLUTTER_DEPTH
      const flutter = ctx.createOscillator()
      flutter.type = 'triangle'
      flutter.frequency.value = this.oscFreq(4)
      const flutterDepth = ctx.createGain()
      automate(flutterDepth.gain, (rpm) => ENGINE_FLUTTER_DEPTH * putter(rpm))
      flutter.connect(flutterDepth)
      flutterDepth.connect(grain.gain)

      // The saw drives the gate's gain, multiplying the noise by it.
      const rasp = ctx.createBufferSource()
      rasp.buffer = getNoiseBuffer(ctx)
      rasp.loop = true
      const raspGate = ctx.createGain()
      raspGate.gain.value = 0
      this.oscs[0].connect(raspGate.gain)
      const raspLevel = ctx.createGain()
      raspLevel.gain.value = ENGINE_RASP_LEVEL
      rasp.connect(raspGate)
      raspGate.connect(raspLevel)
      raspLevel.connect(filter)

      highpass.connect(grain)
      grain.connect(env)
      texture = { flutter, rasp }
    } else {
      highpass.connect(env)
    }
    env.connect(revFilter)
    revFilter.connect(snarl)
    snarl.connect(revGain)
    revGain.connect(level)
    let bus: AudioNode = out
    if (!this.self) {
      const { bus: crowdInput, engines } = crowdBus(ctx, out)
      bus = crowdInput
      engines.add(this)
      updateCrowdGain(ctx)
    }
    if (ctx.createStereoPanner) {
      this.panner = ctx.createStereoPanner()
      this.panner.pan.value = pan
      level.connect(this.panner)
      this.panner.connect(bus)
    } else {
      level.connect(bus)
    }

    wobble.start(t)
    drift.start(t)
    wander.start(t, Math.random() * getWanderBuffer(ctx).duration)
    if (texture) {
      texture.rasp.start(t, Math.random() * 0.5)
      this.oscs.push(texture.flutter)
    }
    this.oscs.forEach((osc) => {
      const detune = osc.detune.value
      automate(osc.detune, (rpm) => detune + 1200 * Math.log2(rpm))
      osc.start(t)
    })
    this.env = env
    this.level = level
    this.filter = filter
    this.snarl = snarl
    const oscs = this.oscs
    // The engine stays counted on the crowd bus until its tail has played out,
    // so the bus doesn't turn up while engines are winding down.
    if (!this.self) {
      oscs[0].onended = () => {
        crowd?.engines.delete(this)
        updateCrowdGain(ctx)
      }
    }
    this.nodes = {
      wobble,
      stop: (at) => {
        wobble.stop(at)
        drift.stop(at)
        wander.stop(at)
        starter?.stop(Math.min(at, t + sim.crankEnd + 0.3))
        popper?.stop(Math.min(at, t + sim.crankEnd + 0.2))
        oscs.forEach((osc) => osc.stop(at))
        texture?.rasp.stop(at)
      },
    }
  }

  setPan(pan: number) {
    if (!ctx || !this.panner) return
    this.panner.pan.setTargetAtTime(Math.max(-1, Math.min(1, pan)), ctx.currentTime, 0.05)
  }

  // Sets how close this engine is to the listener, from 0 (across the arena)
  // to 1 (right beside them).
  setProximity(proximity: number) {
    proximity = Math.max(0, Math.min(1, proximity))
    if (Math.abs(proximity - this.proximity) < 0.01) return
    this.proximity = proximity
    if (!ctx || this.level === null || this.filter === null) return
    const t = ctx.currentTime
    this.level.gain.setTargetAtTime(this.levelGain(), t, 0.1)
    this.filter.frequency.setTargetAtTime(this.cutoff(), t, 0.1)
    if (!this.self) updateCrowdGain(ctx)
  }

  // This engine's power relative to one at full level, for the crowd bus.
  power() {
    return this.levelGain() ** 2
  }

  private levelGain() {
    return 0.4 + 0.6 * this.proximity
  }

  private cutoff() {
    return this.voice.cutoff * (0.55 + 0.45 * this.proximity)
  }

  private oscFreq(index: number) {
    return this.baseFreq * ENGINE_OSC_RATIOS[index] * this.pitch * this.doppler
  }

  // Shifts the engine's pitch by a Doppler factor, above 1 when the engine is
  // closing on the listener.
  setDoppler(factor: number) {
    if (Math.abs(factor - this.doppler) < 0.002) return
    this.doppler = factor
    if (!ctx || this.nodes === null) return
    const t = ctx.currentTime
    this.oscs.forEach((osc, i) => osc.frequency.setTargetAtTime(this.oscFreq(i), t, 0.05))
  }

  // Pitches the engine up in proportion to how far the speed is above normal
  // towards the full slipstream boost.
  setSpeed(speed: number) {
    const boost = Math.max(0, Math.min(1, (speed - LIGHTCYCLE_SPEED) / (LIGHTCYCLE_BOOST_SPEED - LIGHTCYCLE_SPEED)))
    const pitch = 1 + (ENGINE_BOOST_PITCH - 1) * boost
    if (Math.abs(pitch - this.pitch) < 0.001) return
    this.pitch = pitch
    if (!ctx || this.nodes === null) return
    const t = ctx.currentTime
    this.oscs.forEach((osc, i) => osc.frequency.setTargetAtTime(this.oscFreq(i), t, 0.08))
  }

  // Brief pitch rise and snarl, used when the cycle turns.
  rev() {
    if (!ctx || this.nodes === null) return
    const t = ctx.currentTime
    this.oscs.forEach((osc, i) => {
      const freq = this.oscFreq(i)
      osc.frequency.cancelScheduledValues(t)
      osc.frequency.setValueAtTime(freq * 1.25, t)
      osc.frequency.exponentialRampToValueAtTime(freq, t + 0.25)
    })
    if (this.snarl !== null && t >= this.revUpEnds) {
      const gain = this.snarl.gain
      gain.cancelScheduledValues(t)
      gain.setTargetAtTime(ENGINE_TURN_SNARL_DB, t, 0.015)
      gain.setTargetAtTime(0, t + 0.08, 0.2)
    }
  }

  // Stops the engine: cut off at once, or with throttleDown, wound down over
  // about two seconds as if the throttle were released.
  stop(throttleDown = false) {
    if (!ctx || this.nodes === null || this.env === null || this.filter === null) return
    const t = ctx.currentTime
    const gain = this.env.gain
    gain.cancelScheduledValues(t)
    if (throttleDown) {
      const winddown = 1.6
      this.oscs.forEach((osc) => {
        osc.frequency.cancelScheduledValues(t)
        osc.frequency.setValueAtTime(osc.frequency.value, t)
        osc.frequency.exponentialRampToValueAtTime(osc.frequency.value * 0.4, t + winddown)
      })
      this.nodes.wobble.frequency.setTargetAtTime(this.voice.wobbleRate * 0.4, t, winddown / 3)
      this.filter.frequency.setTargetAtTime(this.cutoff() * 0.35, t, winddown / 3)
      gain.setValueAtTime(gain.value, t)
      gain.setTargetAtTime(0, t + 0.15, winddown / 4)
      this.nodes.stop(t + winddown + 0.4)
    } else {
      gain.setTargetAtTime(0, t, 0.05)
      this.nodes.stop(t + 0.4)
    }
    if (this.slot !== null) takenSlots.delete(this.slot)
    this.slot = null
    this.nodes = null
    this.env = null
    this.level = null
    this.filter = null
    this.snarl = null
  }
}

// Takes the first free pitch slot at or after the preferred one.
function takeSlot(preference: number) {
  for (let i = 0; i < ENGINE_SLOT_RATIOS.length; i++) {
    const slot = (preference + i) % ENGINE_SLOT_RATIOS.length
    if (!takenSlots.has(slot)) {
      takenSlots.add(slot)
      return slot
    }
  }
  return preference
}
