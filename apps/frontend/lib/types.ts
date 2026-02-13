// TypeScript types for backend API responses

export type GameState =
  | 'LOBBY'
  | 'BIDDING'
  | 'TRUMP_SELECTION'
  | 'TRICK_PLAY'
  | 'SCORING'
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
  viewer_is_member?: boolean
  viewer_is_host?: boolean
  can_rejoin?: boolean
  viewer_is_turn?: boolean
}

export interface GameListResponse {
  games: Game[]
}

export interface LastActiveGameResponse {
  game_ids: number[]
}
