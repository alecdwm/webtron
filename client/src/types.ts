// Shapes of the data sent by the server (see src/server in the repo root).

export type Color = 'blue' | 'green' | 'orange' | 'purple' | 'red' | 'white'
export type Direction = 'up' | 'down' | 'left' | 'right'
export type Point = [number, number]

// The arena's shape for a round, and the rules changing how it plays. See
// src/server/arena/round.rs.
export type Shape = 'Grid' | 'Circle' | 'Pillars' | 'Cross' | 'Core' | 'Bastions'
export type Mutator = 'Shrink' | 'Wrap' | 'Snake' | 'Overdrive' | 'Fog' | 'Obstacles'
export type Round = {
  shape: Shape
  mutators: Mutator[]
  // Walled-off layout cells, as [column, row] from the bottom left.
  blocked: [number, number][]
}

export type Emote = 'gg' | 'lol' | 'rip' | 'wtf'

export type Player = {
  name: string
  color: Color
  npc?: boolean
}

// What a lightcycle crashed into: a player's trail (possibly its own), the
// arena boundary, or another player's lightcycle head-on.
export type DeathCause = { Trail: string } | 'Boundary' | { Lightcycle: string }

export type ServerLightcycle = {
  position: Point
  direction: Direction
  speed: number
  dead: boolean
  death_cause: DeathCause | null
  // How much braking is left, from 0 to 1.
  brake: number
  // Whether the player is holding the brake, which only slows the lightcycle
  // while there is brake left.
  braking: boolean
}

// A trail can be cut into several paths by holes. The last path is the one
// its lightcycle is still laying down, and its last point follows the
// lightcycle.
export type ServerLightribbon = {
  paths: Point[][]
}

export type Hole = {
  player_id: string
  position: Point
  radius: number
}

export type ServerArena = {
  id: string
  name: string
  width: number
  height: number
  max_players: number

  // ISO 8601 time on the server's clock.
  started: string | null
  winner: string | null

  players: Record<string, Player>
  lightcycles: Record<string, ServerLightcycle>
  lightribbons: Record<string, ServerLightribbon>
  holes: Hole[]
  round: Round
}

// One change to an arena, in serde's externally tagged enum format.
export type ArenaUpdate =
  | { AddPlayer: [string, Player] }
  | { AddLightcycle: [string, ServerLightcycle] }
  | { AddLightribbon: [string, ServerLightribbon] }
  | { AddHole: Hole }
  | { Start: string }
  | 'End'
  | { SetRound: Round }
  | { SetWinner: string | null }
  | { UpdateLightcyclePosition: [string, Point] }
  | { UpdateLightcycleDirection: [string, Direction] }
  | { UpdateLightcycleSpeed: [string, number] }
  | { UpdateLightcycleApplyDeath: [string, DeathCause] }
  | { UpdateLightcycleBraking: [string, boolean] }
  | { UpdateLightcycleBrakeMeter: [string, number] }
  | { UpdateLightribbonAppendPoint: [string, Point] }
  | { UpdateLightribbonReplaceLatestPoint: [string, Point] }
  | { UpdateLightribbonStartPath: [string, Point] }
  | { TrimLightribbon: [string, number] }
  | { NearMiss: [string, Point] }
  | { Emote: [string, Emote] }
  | { RemovePlayer: string }
  | { RemoveLightcycle: string }
  | { RemoveLightribbon: string }
  | { DerezLightribbon: string }

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
  // Human players and NPCs killed.
  kills: number
  // The colour of the player's last win, if known.
  color: Color | null
}

export type Leaderboard = {
  top: LeaderboardEntry[]
  // The viewing player's own entry, whether or not it is in top.
  player: LeaderboardEntry | null
}

export type ServerMessage =
  | { ArenaList: ArenaOverview[] }
  | { ArenaJoined: [string, string] }
  | 'NameInUse'
  // time is the Unix time (ms) on the server's clock of the update the state
  // or changes come from.
  | { ArenaState: { time: number; state: ServerArena } }
  | { ArenaStatePatch: { time: number; updates: ArenaUpdate[] } }
  | { Leaderboard: Leaderboard }

export type ClientMessage =
  | { ClaimName: string }
  | { GetArenaList: null }
  | { Join: { player: Player; arena_id: string | null } }
  | { Start: null }
  | { Turn: Direction }
  | { Brake: boolean }
  | { Emote: Emote }

// The client's copies of the server's data.

export type Lightcycle = ServerLightcycle & {
  // The Unix time (ms) on the server's clock of the update that set the
  // lightcycle's position.
  updatedAt: number
}

// A trail split into the points that stay put and the one that follows the
// lightcycle, so the settled part can be drawn once and only the end redrawn
// as it moves.
export type Lightribbon = {
  // Every path of the trail, less the last point of the last path.
  fixed: Point[][]
  // The last point of the last path, or null for a trail with no points.
  tip: Point | null
}

export type Arena = {
  id: string
  name: string
  width: number
  height: number
  max_players: number

  // Unix time (ms) on the server's clock at which the round starts, or null
  // between rounds.
  started: number | null
  winner: string | null

  players: Record<string, Player>
  lightcycles: Record<string, Lightcycle>
  lightribbons: Record<string, Lightribbon>
  holes: Hole[]
  // Dead trails the server has removed, kept until the next round so they can
  // be animated out.
  derezzing: Record<string, Lightribbon>
  round: Round
  // The walled-off cells of the round before this one, which morph into
  // this round's as it counts down.
  previousBlocked: [number, number][]
  // The latest near misses and each player's latest emote. Each has a number
  // of its own, so a component can play its animation once.
  nearMisses: NearMiss[]
  emotes: Record<string, EmoteShown>
}

export type NearMiss = { key: number; player: string; point: Point }
export type EmoteShown = { key: number; emote: Emote }

export type Stage = 'MainMenu' | 'Connect' | 'ArenaSelect' | 'Arena'
export type SocketState = 'NOT_CONNECTED' | 'CONNECTING' | 'OPEN' | 'CLOSING' | 'CLOSED'

export type PlayerSettings = { id: string | null; name: string; color: Color }

// How the player turns their lightcycle: 'directional' turns it to face the
// arrow pressed, 'steering' turns it left or right of the way it is going.
export type ControlScheme = 'directional' | 'steering'
export type Side = 'left' | 'right'

export type Store = {
  arena: Arena | null
  arenaList: ArenaOverview[] | null
  controlScheme: ControlScheme
  leaderboard: Leaderboard | null
  joinError: string | null
  player: PlayerSettings
  // The server's clock minus the client's (performance.now()), in
  // milliseconds.
  serverTimeOffset: number
  socketState: SocketState
  stage: Stage
}

export type Action =
  | { type: 'INIT_STORE' }
  | { type: 'SET_STAGE'; stage: Stage }
  | { type: 'SET_PLAYER_NAME'; name: string }
  | { type: 'SET_PLAYER_COLOR'; color: Color }
  | { type: 'SET_CONTROL_SCHEME'; controlScheme: ControlScheme }
  | { type: 'SET_SOCKET_STATE'; socketState: SocketState }
  | { type: 'RECEIVE_SERVER_TIME'; serverTimeOffset: number }
  | { type: 'RECEIVE_ARENA_LIST'; arenaList: ArenaOverview[] }
  | { type: 'RECEIVE_ARENA_JOINED'; arenaId: string; playerId: string }
  | { type: 'RECEIVE_NAME_IN_USE' }
  // time is the Unix time (ms) on the server's clock of the update the state
  // or changes come from.
  | { type: 'RECEIVE_ARENA_STATE'; state: ServerArena; time: number }
  | { type: 'RECEIVE_ARENA_STATE_PATCH'; statePatch: ArenaUpdate[]; time: number }
  | { type: 'RECEIVE_LEADERBOARD'; leaderboard: Leaderboard }

export type Thunk = (dispatch: Dispatch, getState: () => Store) => void
export type Dispatch = (action: Action | Thunk) => void
