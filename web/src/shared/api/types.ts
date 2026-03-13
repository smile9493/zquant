export interface PriceBar {
  time: number
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

export interface Job {
  id: string
  status: 'pending' | 'running' | 'completed' | 'failed'
  created_at: string
}

export interface LogEntry {
  timestamp: string
  level: 'info' | 'warn' | 'error'
  message: string
}

export interface DataSource {
  id: string
  name: string
}

export interface DataSet {
  id: string
  name: string
  source_id: string
}
