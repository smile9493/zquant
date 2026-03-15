import type { TypedWsMessage, JobCreatedPayload, JobStartedPayload, JobCompletedPayload } from './protocol'
import type { JobSummary, LogEntry } from '../api/types'

function isJobSummary(obj: any): obj is JobSummary {
  return obj &&
    typeof obj.job_id === 'string' &&
    typeof obj.job_type === 'string' &&
    typeof obj.status === 'string' &&
    typeof obj.stop_requested === 'boolean' &&
    typeof obj.created_at === 'string' &&
    typeof obj.updated_at === 'string'
}

function isJobCreatedPayload(obj: any): obj is JobCreatedPayload {
  return obj && typeof obj.job_id === 'string' && typeof obj.job_type === 'string' && typeof obj.created_at === 'string'
}

function isJobStartedPayload(obj: any): obj is JobStartedPayload {
  return obj && typeof obj.job_id === 'string' && typeof obj.executor_id === 'string' && typeof obj.lease_until_ms === 'number'
}

function isJobCompletedPayload(obj: any): obj is JobCompletedPayload {
  return obj && typeof obj.job_id === 'string' && typeof obj.status === 'string' && typeof obj.duration_ms === 'number'
}

function isLogEntry(obj: any): obj is LogEntry {
  return obj && typeof obj.timestamp === 'string' && typeof obj.level === 'string' && typeof obj.message === 'string'
}

export function decodeWsMessage(raw: unknown): TypedWsMessage | null {
  if (!raw || typeof raw !== 'object') return null

  const msg = raw as any
  if (typeof msg.v !== 'number' || typeof msg.type !== 'string' || typeof msg.ts !== 'string') {
    return null
  }

  if (msg.type === 'hello') {
    return { v: msg.v, type: 'hello', ts: msg.ts, data: msg.data || {} }
  }

  if (msg.type === 'snapshot') {
    const jobs = msg.data?.jobs
    if (jobs && (!Array.isArray(jobs) || !jobs.every(isJobSummary))) {
      return null
    }
    return { v: msg.v, type: 'snapshot', ts: msg.ts, data: { health: msg.data?.health, jobs: jobs || [] } }
  }

  if (msg.type === 'event') {
    const kind = msg.data?.kind
    const payload = msg.data?.payload

    if (kind === 'job.created' && isJobCreatedPayload(payload)) {
      return { v: msg.v, type: 'event', ts: msg.ts, data: { kind, payload } }
    }
    if (kind === 'job.started' && isJobStartedPayload(payload)) {
      return { v: msg.v, type: 'event', ts: msg.ts, data: { kind, payload } }
    }
    if (kind === 'job.completed' && isJobCompletedPayload(payload)) {
      return { v: msg.v, type: 'event', ts: msg.ts, data: { kind, payload } }
    }
    return null
  }

  if (msg.type === 'log') {
    const job_id = msg.data?.job_id
    const entry = msg.data?.entry
    if (typeof job_id === 'string' && isLogEntry(entry)) {
      return { v: msg.v, type: 'log', ts: msg.ts, data: { job_id, entry } }
    }
    return null
  }

  return null
}
