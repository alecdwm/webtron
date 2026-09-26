import type { Dayjs } from 'dayjs'

// Shapes of the data sent by the server (see src/server in the repo root).

export type Color = 'blue' | 'green' | 'orange' | 'purple' | 'red' | 'white'
export type Direction = 'up' | 'down' | 'left' | 'right'
export type Point = [number, number]

export type Player = {
  name: string
  color: Color
  npc?: boolean
}

export type Lightcycle = {
  position: Point
  direction: Direction
  speed: number
  dead: boolean
}

// A trail can be cut into several paths by holes. The last path is the one
// its lightcycle is still laying down.
export type Lightribbon = {
  paths: Point[][]
}

export type Hole = {
  player_id: string
  position: Point
  radius: number
}

export type Arena = {
  id: string
  name: string
  width: number
  height: number
  max_players: number

  started: Dayjs | null
  winner: string | null

  players: Record<string, Player>
  lightcycles: Record<string, Lightcycle>
  lightribbons: Record<string, Lightribbon>
  holes: Hole[]
  // Client-side only: dead trails the server has removed, kept until the next
  // round so they can be animated out.
  derezzing: Record<string, Lightribbon>
}

export type ArenaOverview = {
  id: string
  name: string
  max_players: number
  started: string | null
  players: Record<string, Player>
}

export type LeaderboardEntry = {
  // null for a player with no wins.
  rank: number | null
  name: string
  wins: number
  // The colour of the player's last win, if known.
  color: Color | null
}

export type Leaderboard = {
  top: LeaderboardEntry[]
  // The viewing player's own entry, whether or not it is in top.
  player: LeaderboardEntry | null
}

export type Store = {
  arena: Arena | null
  arenaList: ArenaOverview[] | null
  leaderboard: Leaderboard | null
  config: { debugReducers: boolean }
  player: { id: string | null; name: string; color: Color }
  preloadedImages: HTMLImageElement[]
  socketState: string
  stage: string
}
