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
