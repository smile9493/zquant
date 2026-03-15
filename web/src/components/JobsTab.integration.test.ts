import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useJobStore } from '../stores/jobs'

describe('JobsTab integration - WS disconnected scenarios', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('should preserve optimistic stop when WS disconnected', () => {
    const store = useJobStore()

    // Simulate HTTP data (WS disconnected)
    const httpJobs = [
      { job_id: 'job1', status: 'running', stop_requested: false } as any
    ]
    store.setJobs(httpJobs)

    // User clicks stop
    store.applyOptimisticStop('job1')

    // Verify optimistic overlay is visible even without WS
    expect(store.displayJobs[0].stop_requested).toBe(true)
    expect(store.displayJobs[0].job_id).toBe('job1')
  })

  it('should preserve optimistic retry when WS disconnected', () => {
    const store = useJobStore()

    // Simulate HTTP data (WS disconnected)
    const httpJobs = [
      { job_id: 'job1', status: 'error' } as any
    ]
    store.setJobs(httpJobs)

    // User clicks retry
    store.applyOptimisticRetry('job1')

    // Verify optimistic overlay is visible
    expect((store.displayJobs[0] as any)._retrying).toBe(true)
  })

  it('should show optimistic new job when WS disconnected', () => {
    const store = useJobStore()

    // Simulate HTTP data (WS disconnected)
    const httpJobs = [
      { job_id: 'job1', status: 'error' } as any
    ]
    store.setJobs(httpJobs)

    // Retry succeeds, add optimistic new job
    const newJob = {
      job_id: 'job2',
      job_type: 'test',
      status: 'queued',
      stop_requested: false,
      created_at: '2026-03-15T00:00:00Z',
      updated_at: '2026-03-15T00:00:00Z'
    } as any

    store.addOptimisticJob(newJob)

    // Verify new job appears at top even without WS
    expect(store.displayJobs).toHaveLength(2)
    expect(store.displayJobs[0].job_id).toBe('job2')
    expect(store.displayJobs[0].status).toBe('queued')
  })

  it('should handle HTTP data update while optimistic action pending', () => {
    const store = useJobStore()

    // Initial HTTP data
    store.setJobs([
      { job_id: 'job1', status: 'running', stop_requested: false } as any
    ])

    // User clicks stop
    store.applyOptimisticStop('job1')
    expect(store.displayJobs[0].stop_requested).toBe(true)

    // HTTP refresh arrives (stop not yet reflected)
    store.setJobs([
      { job_id: 'job1', status: 'running', stop_requested: false } as any
    ])

    // Optimistic overlay should still be visible
    expect(store.displayJobs[0].stop_requested).toBe(true)
  })
})
