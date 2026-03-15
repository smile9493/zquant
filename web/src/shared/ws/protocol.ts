import type { JobSummary, LogEntry } from '../api/types'

export interface HelloMessage {
  v: number
  type: 'hello'
  ts: string
  data: {
    server?: string
    schema_v?: string
  }
}

export interface SnapshotMessage {
  v: number
  type: 'snapshot'
  ts: string
  data: {
    health?: unknown
    jobs?: JobSummary[]
  }
}

// Backend event payloads (actual WS protocol)
export interface JobCreatedPayload {
  job_id: string
  job_type: string
  created_at: string
}

export interface JobStartedPayload {
  job_id: string
  executor_id: string
  lease_until_ms: number
}

export interface JobCompletedPayload {
  job_id: string
  status: string
  duration_ms: number
  error?: unknown
  artifacts?: unknown
}

export type JobEventPayload = JobCreatedPayload | JobStartedPayload | JobCompletedPayload

export interface JobEventMessage {
  v: number
  type: 'event'
  ts: string
  data: {
    kind: 'job.created' | 'job.started' | 'job.completed'
    payload: JobEventPayload
  }
}

export interface LogMessage {
  v: number
  type: 'log'
  ts: string
  data: {
    job_id: string
    entry: LogEntry
  }
}

export type TypedWsMessage = HelloMessage | SnapshotMessage | JobEventMessage | LogMessage
