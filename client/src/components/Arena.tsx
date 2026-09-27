import { type CSSProperties, type RefObject, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

import { brake, emote, start, steer, turn } from '@/actions'
import { playClick } from '@/audio/sound'
import ArenaGrid from '@/components/ArenaGrid'
import ArenaLayout from '@/components/ArenaLayout'
import Countdown from '@/components/Countdown'
import Fog from '@/components/Fog'
import Hole from '@/components/Hole'
import Leaderboard from '@/components/Leaderboard'
import Lightcycle from '@/components/Lightcycle'
import Lightribbon from '@/components/Lightribbon'
import MenuButton from '@/components/MenuButton'
import NearMisses from '@/components/NearMisses'
import Notifications from '@/components/Notifications'
import PortalWindows from '@/components/PortalWindows'
import RoundBanner from '@/components/RoundBanner'
import ScenarioPanel from '@/components/ScenarioPanel'
import useArenaAnimation, { ArenaAnimationContext, EMOTE_MS } from '@/hooks/useArenaAnimation'
import useArenaSounds from '@/hooks/useArenaSounds'
import useEventListener from '@/hooks/useEventListener'
import usePreloadImages from '@/hooks/usePreloadImages'
import useStore from '@/hooks/useStore'
import useStoreDispatch from '@/hooks/useStoreDispatch'
import backgroundPanel from '@/img/background-panel.svg'
import type { Arena as ArenaType, Color, ControlScheme, Direction, Emote, EmoteShown, Scenario, Side } from '@/types'
import { serverNow } from '@/utils/clock'
import { colorToCss } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'
import resolveClassName from '@/utils/resolveClassName'
import { emoteLabels, emotes, hasMutator } from '@/utils/round'
import { readScenario, storeScenario } from '@/utils/scenario'
import { spawnDelayMs } from '@/utils/spawn'

import styles from './Arena.module.css'

export default function Arena() {
  usePreloadImages([backgroundPanel, ...Object.values(lightcycleImages)])
  const arena = useStore((store) => store.arena)
  return arena ? <ArenaView arena={arena} /> : null
}

function ArenaView({ arena }: { arena: ArenaType }) {
  const leaderboard = useStore((store) => store.leaderboard)
  const player = useStore((store) => store.player)
  const controlScheme = useStore((store) => store.controlScheme)
  const serverTimeOffset = useStore((store) => store.serverTimeOffset)
  useArenaSounds(arena, player.id, serverTimeOffset)
  const arenaRef = useRef<HTMLDivElement>(null)
  const animation = useArenaAnimation(arena, serverTimeOffset, player.id, arenaRef)
  const dispatch = useStoreDispatch()

  // A test scenario for the next round, set up in the hidden scenario panel,
  // which is open while there is one.
  const [scenario, setScenario] = useState<Scenario | null>(readScenario)
  const changeScenario = useCallback((scenario: Scenario | null) => {
    setScenario(scenario)
    storeScenario(scenario)
  }, [])
  const onHintTap = useSecretTaps(() => changeScenario(scenario ?? { shape: null, mutators: null, npcs: null }))
  const onStart = useCallback(() => dispatch(start(scenario)), [dispatch, scenario])

  const inRound = arena.started !== null
  const self = player.id !== null ? arena.lightcycles[player.id] : undefined
  const selfAlive = inRound && self !== undefined && !self.dead
  useKeyControls(inRound, selfAlive, controlScheme)
  useTouchControls(inRound, controlScheme, arenaRef)
  useEmoteKeys()
  useCaptureGameKeys()

  const { width, height } = arena
  const colorOf = (id: string): Color => arena.players[id]?.color ?? 'white'
  const winner = arena.winner ? arena.players[arena.winner] : undefined
  const playerCount = Object.keys(arena.players).length
  // The player count is only meaningful before the first round: afterwards
  // it also counts NPCs from the round that just ended.
  const roundPlayed = Object.keys(arena.lightcycles).length > 0
  // Once every human is out, anyone can start the next round without
  // waiting for the NPCs to finish this one.
  const humanAlive = Object.entries(arena.lightcycles).some(([id, { dead }]) => !dead && !arena.players[id]?.npc)
  const canCutShort = inRound && roundPlayed && !humanAlive
  const showLobby = !inRound || canCutShort
  const startLabel = winner || canCutShort ? 'REMATCH' : 'START'
  const spawning = arena.started !== null && serverNow(serverTimeOffset) < arena.started
  useStartKeys(showLobby, onStart)

  return (
    <ArenaAnimationContext.Provider value={animation}>
      <div ref={arenaRef} className={styles.arena}>
        <ArenaGrid startAt={arena.started} serverTimeOffset={serverTimeOffset} />
        <ArenaLayout
          round={arena.round}
          previousBlocked={arena.previousBlocked}
          startAt={arena.started}
          serverTimeOffset={serverTimeOffset}
          width={width}
        />

        {arena.holes.map(({ player_id, position, radius }) => (
          <Hole
            key={player_id}
            color={colorOf(player_id)}
            position={position}
            radius={radius}
            width={width}
            height={height}
          />
        ))}
        {Object.entries(arena.lightribbons).map(([id, { fixed }]) => (
          <Lightribbon key={id} id={id} color={colorOf(id)} fixed={fixed} width={width} height={height} />
        ))}
        <PortalWindows
          lightribbons={arena.lightribbons}
          round={arena.round}
          width={width}
          height={height}
          colorOf={colorOf}
        />
        {Object.entries(arena.derezzing).map(([id, { fixed, tip }]) => (
          <Lightribbon
            key={`derez-${id}`}
            color={colorOf(id)}
            fixed={fixed}
            tip={tip}
            width={width}
            height={height}
            derezzing
          />
        ))}
        {Object.entries(arena.lightcycles).map(([id, { direction, speed, dead, braking, brake }], index) => (
          <Lightcycle
            key={id}
            id={id}
            name={arena.players[id]?.name ?? 'DISCONNECTED'}
            color={colorOf(id)}
            direction={direction}
            speed={speed}
            dead={dead}
            isSelf={id === player.id}
            inLobby={!inRound}
            spawning={spawning}
            spawnDelay={spawnDelayMs(index)}
            braking={braking && brake > 0}
            brake={id === player.id && inRound ? brake : null}
            emote={dead ? undefined : recentEmote(arena.emotes[id], serverTimeOffset)}
            copies={hasMutator(arena.round, 'Wrap') || arena.round.portals.length > 0}
            portals={arena.round.portals.length > 0}
          />
        ))}
        <NearMisses nearMisses={arena.nearMisses} colorOf={colorOf} selfId={player.id} width={width} height={height} />

        {inRound && hasMutator(arena.round, 'Fog') ? <Fog lifted={!selfAlive} /> : null}

        <Notifications arena={arena} />
        <RoundBanner round={arena.round} startAt={arena.started} serverTimeOffset={serverTimeOffset} />
        <EmoteBar color={colorOf(player.id ?? '')} />
        {selfAlive ? <BrakeButton /> : null}

        {arena.started !== null ? (
          <Countdown key={arena.started} startAt={arena.started} serverTimeOffset={serverTimeOffset} />
        ) : null}

        {winner || showLobby ? (
          <div className={resolveClassName([styles.overlay, showLobby && scenario && styles.testing])}>
            {winner && (
              <div className={styles.winner} style={{ '--winner': colorToCss(winner.color) } as CSSProperties}>
                <div className={styles.winnerLabel}>Winner</div>
                <div className={styles.winnerName}>{winner.name.toUpperCase()}</div>
                {leaderboard ? <Leaderboard leaderboard={leaderboard} playerColor={player.color} /> : null}
              </div>
            )}

            {showLobby && scenario ? (
              <ScenarioPanel
                scenario={scenario}
                onChange={changeScenario}
                onClose={() => changeScenario(null)}
                start={
                  <MenuButton className={styles.scenarioStart} onClick={onStart}>
                    {startLabel}
                  </MenuButton>
                }
              />
            ) : showLobby ? (
              <div className={styles.lobby}>
                {!winner && !canCutShort && <div className={styles.arenaName}>{arena.name}</div>}
                <MenuButton className={styles.startButton} onClick={onStart}>
                  {startLabel}
                </MenuButton>
                {!roundPlayed && (
                  <div className={styles.lobbyInfo}>
                    {playerCount}/{arena.max_players} players
                  </div>
                )}
                <div className={styles.controlsHint} onClick={onHintTap}>
                  {controlScheme === 'steering'
                    ? '← → / A D / tap either side to turn'
                    : 'Arrows / WASD / tap to steer'}
                  <br />
                  Space / Shift to brake · 1–3 to emote
                </div>
              </div>
            ) : null}
          </div>
        ) : null}
      </div>
    </ArenaAnimationContext.Provider>
  )
}

const gameKeys = new Set([
  'Space',
  'Enter',
  'NumpadEnter',
  'ShiftLeft',
  'ShiftRight',
  'ArrowUp',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  'KeyW',
  'KeyA',
  'KeyS',
  'KeyD',
  'Digit1',
  'Digit2',
  'Digit3',
  'Digit4',
  'Numpad1',
  'Numpad2',
  'Numpad3',
  'Numpad4',
])

// Keeps the game's keys from scrolling the page or doing anything else the
// browser would do with them, in and between rounds, so a key still held as
// a round ends doesn't scroll it. A focused button still gets a fresh press
// of Space or Enter, and a focused field gets every key.
function useCaptureGameKeys() {
  const onKeyDown = useCallback((event: KeyboardEvent) => {
    if (!gameKeys.has(event.code)) return
    const target = event.target instanceof HTMLElement ? event.target : null
    if (target?.closest('input, textarea, select')) return
    const pressesButton = (event.code === 'Space' || event.code.endsWith('Enter')) && !event.repeat
    if (pressesButton && target?.closest('button')) return
    event.preventDefault()
  }, [])
  useEventListener('keydown', onKeyDown)
}

// How many taps in a row, each within SECRET_TAP_MS of the last, open the
// scenario panel.
const SECRET_TAPS = 5
const SECRET_TAP_MS = 400

// A tap handler that calls `onUnlock` after SECRET_TAPS quick taps.
function useSecretTaps(onUnlock: () => void) {
  const taps = useRef({ count: 0, at: 0 })
  return useCallback(() => {
    const now = performance.now()
    const count = now - taps.current.at < SECRET_TAP_MS ? taps.current.count + 1 : 1
    taps.current = { count: count >= SECRET_TAPS ? 0 : count, at: now }
    if (count >= SECRET_TAPS) onUnlock()
  }, [onUnlock])
}

// Enter or Space presses the START / REMATCH button while it is shown.
function useStartKeys(enabled: boolean, onStart: () => void) {
  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (!enabled || event.repeat || (event.key !== 'Enter' && event.key !== ' ')) return
      // A focused button or field already handles these keys itself.
      if (event.target instanceof HTMLElement && event.target.closest('button, input, textarea, select')) return
      event.preventDefault()
      playClick()
      onStart()
    },
    [enabled, onStart],
  )
  useEventListener('keydown', onKeyDown)
}

// Keyed by the physical key, so WASD works with Caps Lock or Shift held and on
// any keyboard layout.
const turnKeys: Record<string, Direction> = {
  ArrowUp: 'up',
  KeyW: 'up',
  ArrowLeft: 'left',
  KeyA: 'left',
  ArrowRight: 'right',
  KeyD: 'right',
  ArrowDown: 'down',
  KeyS: 'down',
}
const steerKeys: Record<string, Side> = {
  ArrowLeft: 'left',
  KeyA: 'left',
  ArrowRight: 'right',
  KeyD: 'right',
}

// Directional controls turn the lightcycle to face the given direction.
// Steering controls turn it to its left or right.
function useControls(controlScheme: ControlScheme) {
  const dispatch = useStoreDispatch()
  return useMemo(
    () => ({
      onDirection: (direction: Direction) => dispatch(turn(direction)),
      onSide: (side: Side) => dispatch(steer(side)),
      steering: controlScheme === 'steering',
    }),
    [dispatch, controlScheme],
  )
}

const brakeKeys = new Set(['Space', 'ShiftLeft', 'ShiftRight'])

function useKeyControls(inRound: boolean, selfAlive: boolean, controlScheme: ControlScheme) {
  const { onDirection, onSide, steering } = useControls(controlScheme)
  const dispatch = useStoreDispatch()

  // The brake is let go when the player is out, or when the window loses
  // focus and would miss the key coming up.
  useEffect(() => {
    if (!selfAlive) dispatch(brake(false))
  }, [dispatch, selfAlive])
  useEventListener(
    'blur',
    useCallback(() => dispatch(brake(false)), [dispatch]),
  )
  useEventListener(
    'keyup',
    useCallback(
      (event: KeyboardEvent) => {
        if (brakeKeys.has(event.code)) dispatch(brake(false))
      },
      [dispatch],
    ),
  )

  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (!inRound) return

      // Otherwise Space is left to START / REMATCH.
      if (brakeKeys.has(event.code) && selfAlive) {
        event.preventDefault()
        if (!event.repeat) dispatch(brake(true))
        return
      }

      const direction = turnKeys[event.code]
      if (!direction) return
      event.preventDefault()
      // A held key would otherwise send the same turn over and over.
      if (event.repeat) return
      if (!steering) return onDirection(direction)
      const side = steerKeys[event.code]
      if (side) onSide(side)
    },
    [inRound, selfAlive, dispatch, onDirection, onSide, steering],
  )
  useEventListener('keydown', onKeyDown)
}

// The number keys 1 to 3, on either the main keyboard or the number pad,
// pick the emotes in order.
const emoteKeys: Record<string, Emote> = Object.fromEntries(
  emotes.flatMap((chosen, index) => [
    [`Digit${index + 1}`, chosen],
    [`Numpad${index + 1}`, chosen],
  ]),
)

// The number keys emote, in or out of a round.
function useEmoteKeys() {
  const dispatch = useStoreDispatch()
  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      const chosen = emoteKeys[event.code]
      if (!chosen || event.repeat) return
      if (event.target instanceof HTMLElement && event.target.closest('input, textarea, select')) return
      dispatch(emote(chosen))
    },
    [dispatch],
  )
  useEventListener('keydown', onKeyDown)
}

// The emote, if it's still showing. One from before a lightcycle was placed
// for a new round isn't shown again over it.
function recentEmote(emote: EmoteShown | undefined, serverTimeOffset: number) {
  return emote && serverNow(serverTimeOffset) - emote.at < EMOTE_MS ? emote : undefined
}

// A button for each emote, labelled with its key, for touchscreens and mice.
// They're in the player's color, and flash filled in when pressed, like the
// bubble the emote puts over the player's lightcycle.
// The arena is drawn in the page's <main>, which clips it, so the bar goes in
// an element of its own added just after it.
function EmoteBar({ color }: { color: Color }) {
  const dispatch = useStoreDispatch()
  const [container] = useState(() => {
    const element = document.createElement('div')
    element.className = styles.belowArena
    return element
  })
  useEffect(() => {
    document.querySelector('main')?.after(container)
    return () => container.remove()
  }, [container])

  // The last emote pressed and how many presses there have been, which
  // alternates between two identical animations so each press restarts it.
  const [fired, setFired] = useState<{ emote: Emote; count: number } | null>(null)

  return createPortal(
    <div className={styles.emoteBar} style={{ '--cycle': colorToCss(color) } as CSSProperties}>
      {emotes.map((chosen, index) => {
        const flash = fired?.emote === chosen ? (fired.count % 2 ? styles.firedA : styles.firedB) : ''
        return (
          <button
            key={chosen}
            className={`${styles.emoteButton} ${flash}`}
            aria-label={`Emote ${emoteLabels[chosen]} (key ${index + 1})`}
            // Not focused by a click, so Space still brakes rather than pressing
            // the last emote again.
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => {
              dispatch(emote(chosen))
              setFired((last) => ({ emote: chosen, count: (last?.count ?? 0) + 1 }))
            }}
          >
            <span className={styles.emoteLabel}>{emoteLabels[chosen]}</span>
            <kbd className={styles.emoteKey}>{index + 1}</kbd>
          </button>
        )
      })}
    </div>,
    container,
  )
}

// A brake to hold down on touchscreens, which have no keys for it.
function BrakeButton() {
  const dispatch = useStoreDispatch()
  const release = useCallback(() => dispatch(brake(false)), [dispatch])
  return (
    <button
      className={styles.brakeButton}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId)
        dispatch(brake(true))
      }}
      onPointerUp={release}
      onPointerCancel={release}
      onContextMenu={(event) => event.preventDefault()}
    >
      BRAKE
    </button>
  )
}

// Presses on a button are left to it: cancelling the touch would also cancel
// the button's click.
function isOnButton(target: EventTarget | null) {
  return target instanceof Element && target.closest('button') !== null
}

// Only touches steer: a mouse click on the arena is more likely to be focusing
// the window than a turn. With directional controls a touch turns towards
// whichever side of the arena's centre it lands on. With steering controls a
// touch on the left half turns left, and on the right half turns right.
function useTouchControls(inRound: boolean, controlScheme: ControlScheme, arenaRef: RefObject<HTMLDivElement | null>) {
  const { onDirection, onSide, steering } = useControls(controlScheme)

  const onTouchStart = useCallback(
    (event: TouchEvent) => {
      if (!inRound || isOnButton(event.target)) return
      const touch = event.changedTouches[0]
      const arena = arenaRef.current
      if (!touch || !arena) return
      event.preventDefault()

      const rect = arena.getBoundingClientRect()
      const xFromCenter = touch.clientX - rect.left - rect.width / 2
      const yFromCenter = touch.clientY - rect.top - rect.height / 2

      if (steering) onSide(xFromCenter < 0 ? 'left' : 'right')
      else if (Math.abs(xFromCenter) < Math.abs(yFromCenter)) onDirection(yFromCenter < 0 ? 'up' : 'down')
      else onDirection(xFromCenter < 0 ? 'left' : 'right')
    },
    [inRound, arenaRef, onDirection, onSide, steering],
  )

  useEventListener('touchstart', onTouchStart, arenaRef)
}
