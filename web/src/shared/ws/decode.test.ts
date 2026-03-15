import { describe, it, expect } from 'vitest'
import { decodeWsMessage } from './decode'

describe('decodeWsMessage', () => {
  it('should decode valid snapshot with jobs array', () => {
    const raw = {
      v: 1,
      type: 'snapshot',
      ts: '2026-03-15T12:00:00Z',
      data: {
        jobs: [
          { job_id: 'j1', job_type: 'test', status: 'queued', stop_requested: false, created_at: '', updated_at: '' }
        ]
      }
    }
    const result = decodeWsMessage(raw)
    expect(result).not.toBeNull()
    expect(result?.type).toBe('snapshot')
  })

  it('should reject snapshot with invalid jobs array', () => {
    const raw = {
      v: 1,
      type: 'snapshot',
      ts: '2026-03-15T12:00:00Z',
      data: { jobs: [{ invalid: 'data' }] }
    }
    expect(decodeWsMessage(raw)).toBeNull()
  })

  it('should reject snapshot with jobs missing required fields', () => {
    const raw = {
      v: 1,
      type: 'snapshot',
      ts: '2026-03-15T12:00:00Z',
      data: {
        jobs: [
          { job_id: 'j1', job_type: 'test', status: 'queued' } // missing stop_requested, created_at, updated_at
        ]
      }
    }
    expect(decodeWsMessage(raw)).toBeNull()
  })

  it('should decode job.created event', () => {
    const raw = {
      v: 1,
      type: 'event',
      ts: '2026-03-15T12:00:00Z',
      data: {
        kind: 'job.created',
        payload: { job_id: 'j1', job_type: 'test', created_at: '2026-03-15T12:00:00Z' }
      }
    }
    const result = decodeWsMessage(raw)
    expect(result).not.toBeNull()
    expect(result?.type).toBe('event')
  })

  it('should decode job.started event', () => {
    const raw = {
      v: 1,
      type: 'event',
      ts: '2026-03-15T12:00:00Z',
      data: {
        kind: 'job.started',
        payload: { job_id: 'j1', executor_id: 'e1', lease_until_ms: 1000 }
      }
    }
    const result = decodeWsMessage(raw)
    expect(result).not.toBeNull()
    expect(result?.type).toBe('event')
  })

  it('should decode job.completed event', () => {
    const raw = {
      v: 1,
      type: 'event',
      ts: '2026-03-15T12:00:00Z',
      data: {
        kind: 'job.completed',
        payload: { job_id: 'j1', status: 'completed', duration_ms: 500 }
      }
    }
    const result = decodeWsMessage(raw)
    expect(result).not.toBeNull()
    expect(result?.type).toBe('event')
  })

  it('should reject event with invalid payload', () => {
    const raw = {
      v: 1,
      type: 'event',
      ts: '2026-03-15T12:00:00Z',
      data: {
        kind: 'job.created',
        payload: { invalid: 'data' }
      }
    }
    expect(decodeWsMessage(raw)).toBeNull()
  })

  it('should decode log message', () => {
    const raw = {
      v: 1,
      type: 'log',
      ts: '2026-03-15T12:00:00Z',
      data: {
        job_id: 'j1',
        entry: { timestamp: '2026-03-15T12:00:00Z', level: 'info', message: 'test' }
      }
    }
    const result = decodeWsMessage(raw)
    expect(result).not.toBeNull()
    expect(result?.type).toBe('log')
  })

  it('should reject log with invalid entry', () => {
    const raw = {
      v: 1,
      type: 'log',
      ts: '2026-03-15T12:00:00Z',
      data: {
        job_id: 'j1',
        entry: { invalid: 'data' }
      }
    }
    expect(decodeWsMessage(raw)).toBeNull()
  })

  it('should return null for malformed message', () => {
    expect(decodeWsMessage(null)).toBeNull()
    expect(decodeWsMessage({})).toBeNull()
    expect(decodeWsMessage({ type: 'unknown' })).toBeNull()
  })

  it('should return null for unknown event kind', () => {
    const raw = {
      v: 1,
      type: 'event',
      ts: '2026-03-15T12:00:00Z',
      data: { kind: 'unknown.kind', payload: {} }
    }
    expect(decodeWsMessage(raw)).toBeNull()
  })
})
