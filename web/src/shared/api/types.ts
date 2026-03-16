export interface PriceBar {
  time: number
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

export type JobStatus = 'queued' | 'running' | 'done' | 'error' | 'stopped' | 'reaped' | string

export interface JobSummary {
  job_id: string
  job_type: string
  status: JobStatus
  stop_requested: boolean
  created_at: string
  updated_at: string
}

export interface LogEntry {
  timestamp: string
  level: 'info' | 'warn' | 'error' | string
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

export type AgentStatus = 'idle' | 'running' | 'success' | 'error'

export interface AgentSession {
  session_id: string
  status: AgentStatus
  symbol?: string
  timeframe?: string
  dataset_id?: string
  job_id?: string
  last_output?: string
  last_updated: string
  error_message?: string
}
