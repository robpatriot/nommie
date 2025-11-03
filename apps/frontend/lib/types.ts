// TypeScript types for backend API responses

export type GameState =
  | 'LOBBY'
  | 'DEALING'
  | 'BIDDING'
  | 'TRUMP_SELECTION'
  | 'TRICK_PLAY'
  | 'SCORING'
  | 'BETWEEN_ROUNDS'
  | 'COMPLETED'
  | 'ABANDONED'

export type GameVisibility = 'PUBLIC' | 'PRIVATE'

export interface Game {
  id: number
  name: string
  state: GameState
  visibility: GameVisibility
  created_by: number
  created_at: string
  updated_at: string
  started_at: string | null
  ended_at: string | null
  current_round: number | null
  player_count: number
  max_players: number
}

export interface GameListResponse {
  games: Game[]
}

export interface LastActiveGameResponse {
  game_id: number | null
}
