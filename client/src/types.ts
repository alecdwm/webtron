import type { Dayjs } from 'dayjs'

// Shapes of the data sent by the server (see src/server in the repo root).

export type Color = 'blue' | 'green' | 'orange' | 'purple' | 'red' | 'white'
export type Direction = 'up' | 'down' | 'left' | 'right'
export type Point = [number, number]

export type Player = {
  name: string
  color: Color
}

export type Lightcycle = {
  position: Point
  direction: Direction
  speed: number
  dead: boolean
}

export type Lightribbon = {
  points: Point[]
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
}

export type ArenaOverview = {
  id: string
  name: string
  max_players: number
  started: string | null
  players: Record<string, Player>
}

export type Store = {
  arena: Arena | null
  arenaList: ArenaOverview[] | null
  config: { debugReducers: boolean }
  player: { id: string | null; name: string; color: Color }
  preloadedImages: HTMLImageElement[]
  socketState: string
  stage: string
}
