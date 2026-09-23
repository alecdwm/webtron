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

// Continuous engine drone for one lightcycle: two detuned saws and a sub
// square through a lowpass filter whose cutoff is wobbled by an LFO.
export class Engine {
  private nodes: { stop: (t: number) => void } | null = null
  private panner: StereoPannerNode | null = null
  private oscs: OscillatorNode[] = []
  private env: GainNode | null = null
  private baseFreq: number
  private pitch = 1

  constructor(
    private self: boolean,
    variant = 0,
  ) {
    this.baseFreq = self ? 55 : 62 + variant * 6
  }

  start(pan = 0) {
    const a = audio()
    if (a === null || this.nodes !== null) return
    const { ctx, out } = a
    const t = ctx.currentTime

    const filter = ctx.createBiquadFilter()
    filter.type = 'lowpass'
    filter.frequency.value = this.self ? 520 : 380
    filter.Q.value = 4

    const lfo = ctx.createOscillator()
    lfo.frequency.value = 6 + Math.random() * 2
    const lfoDepth = ctx.createGain()
    lfoDepth.gain.value = 90
    lfo.connect(lfoDepth)
    lfoDepth.connect(filter.frequency)

    const env = ctx.createGain()
    env.gain.setValueAtTime(0, t)
    env.gain.linearRampToValueAtTime(this.self ? 0.09 : 0.03, t + 0.4)

    const makeOsc = (type: OscillatorType, ratio: number, detune: number) => {
      const osc = ctx.createOscillator()
      osc.type = type
      osc.frequency.value = this.baseFreq * ratio * this.pitch
      osc.detune.value = detune
      osc.connect(filter)
      return osc
    }
    this.oscs = [makeOsc('sawtooth', 1, -8), makeOsc('sawtooth', 1, 8), makeOsc('square', 0.5, 0)]

    filter.connect(env)
    if (ctx.createStereoPanner) {
      this.panner = ctx.createStereoPanner()
      this.panner.pan.value = pan
      env.connect(this.panner)
      this.panner.connect(out)
    } else {
      env.connect(out)
    }

    lfo.start(t)
    this.oscs.forEach((osc) => osc.start(t))
    this.env = env
    this.nodes = {
      stop: (at) => {
        lfo.stop(at)
        this.oscs.forEach((osc) => osc.stop(at))
      },
    }
  }

  setPan(pan: number) {
    if (!ctx || !this.panner) return
    this.panner.pan.setTargetAtTime(Math.max(-1, Math.min(1, pan)), ctx.currentTime, 0.05)
  }

  private oscFreq(index: number) {
    return this.baseFreq * (index === 2 ? 0.5 : 1) * this.pitch
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

  // Brief pitch rise, used when the cycle turns.
  rev() {
    if (!ctx || this.nodes === null) return
    const t = ctx.currentTime
    this.oscs.forEach((osc, i) => {
      const freq = this.oscFreq(i)
      osc.frequency.cancelScheduledValues(t)
      osc.frequency.setValueAtTime(freq * 1.25, t)
      osc.frequency.exponentialRampToValueAtTime(freq, t + 0.25)
    })
  }

  stop() {
    if (!ctx || this.nodes === null || this.env === null) return
    const t = ctx.currentTime
    this.env.gain.cancelScheduledValues(t)
    this.env.gain.setTargetAtTime(0, t, 0.05)
    this.nodes.stop(t + 0.4)
    this.nodes = null
    this.env = null
  }
}
