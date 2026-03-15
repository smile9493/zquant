import type { JobSummary, LogEntry } from '../api/types'
import type { SnapshotMessage, JobEventMessage, LogMessage, JobCompletedPayload } from './protocol'

export function reduceSnapshot(jobs: JobSummary[], msg: SnapshotMessage): JobSummary[] {
  return msg.data.jobs || jobs
}

export function reduceJobEvent(jobs: JobSummary[], msg: JobEventMessage, ts: string): JobSummary[] {
  const { kind, payload } = msg.data
  const idx = jobs.findIndex(j => j.job_id === payload.job_id)

  if (idx < 0) {
    // Job not in list yet, wait for snapshot/HTTP to provide full data
    return jobs
  }

  const updated = [...jobs]
  if (kind === 'job.created') {
    // Already exists, no update needed (created_at is immutable)
    return jobs
  } else if (kind === 'job.started') {
    // Update with executor info (payload has executor_id, lease_until_ms)
    updated[idx] = { ...updated[idx], updated_at: ts }
  } else if (kind === 'job.completed') {
    // Update with completion info (payload has status, duration_ms, error, artifacts)
    const completedPayload = payload as JobCompletedPayload
    updated[idx] = { ...updated[idx], status: completedPayload.status, updated_at: ts }
  }
  return updated
}

export function reduceLog(logs: Map<string, LogEntry[]>, msg: LogMessage): Map<string, LogEntry[]> {
  const { job_id, entry } = msg.data
  const newLogs = new Map(logs)
  if (!newLogs.has(job_id)) {
    newLogs.set(job_id, [])
  }
  newLogs.get(job_id)!.push(entry)
  return newLogs
}
