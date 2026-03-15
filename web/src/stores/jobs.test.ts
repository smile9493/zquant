import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useJobStore } from './jobs'

describe('jobs store optimistic UI', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('should apply optimistic stop', () => {
    const store = useJobStore()
    store.jobs = [{ job_id: 'job1', status: 'running', stop_requested: false } as any]

    store.applyOptimisticStop('job1')

    expect(store.pendingActions.get('job1')).toMatchObject({
      type: 'stopping',
      jobId: 'job1'
    })
    expect(store.displayJobs[0].stop_requested).toBe(true)
  })

  it('should rollback optimistic stop on error', () => {
    const store = useJobStore()
    store.jobs = [{ job_id: 'job1', status: 'running', stop_requested: false } as any]
    store.applyOptimisticStop('job1')

    store.clearOptimistic('job1')

    expect(store.pendingActions.has('job1')).toBe(false)
    expect(store.displayJobs[0].stop_requested).toBe(false)
  })

  it('should apply optimistic retry with UI feedback', () => {
    const store = useJobStore()
    store.jobs = [{ job_id: 'job1', status: 'error' } as any]

    store.applyOptimisticRetry('job1')

    expect(store.pendingActions.get('job1')).toMatchObject({
      type: 'retrying',
      jobId: 'job1'
    })
    expect((store.displayJobs[0] as any)._retrying).toBe(true)
  })

  it('should rollback optimistic retry on error', () => {
    const store = useJobStore()
    store.jobs = [{ job_id: 'job1', status: 'error' } as any]
    store.applyOptimisticRetry('job1')

    store.clearOptimistic('job1')

    expect(store.pendingActions.has('job1')).toBe(false)
    expect((store.displayJobs[0] as any)._retrying).toBeUndefined()
  })

  it('should preserve unrelated pending actions', () => {
    const store = useJobStore()
    store.jobs = [
      { job_id: 'job1', status: 'running' } as any,
      { job_id: 'job2', status: 'running' } as any
    ]
    store.applyOptimisticStop('job1')
    store.applyOptimisticRetry('job2')

    store.clearOptimistic('job1')

    expect(store.pendingActions.has('job1')).toBe(false)
    expect(store.pendingActions.has('job2')).toBe(true)
  })

  it('should not affect jobs without pending actions', () => {
    const store = useJobStore()
    store.jobs = [
      { job_id: 'job1', status: 'running', stop_requested: false } as any,
      { job_id: 'job2', status: 'done' } as any
    ]
    store.applyOptimisticStop('job1')

    expect(store.displayJobs[0].stop_requested).toBe(true)
    expect(store.displayJobs[1]).toEqual(store.jobs[1])
  })

  it('should add optimistic job on retry success', () => {
    const store = useJobStore()
    store.jobs = [{ job_id: 'job1', status: 'error' } as any]

    const newJob = {
      job_id: 'job2',
      job_type: 'test',
      status: 'queued',
      stop_requested: false,
      created_at: '2026-03-15T00:00:00Z',
      updated_at: '2026-03-15T00:00:00Z'
    } as any

    store.addOptimisticJob(newJob)

    expect(store.displayJobs).toHaveLength(2)
    expect(store.displayJobs[0].job_id).toBe('job2')
    expect(store.displayJobs[0].status).toBe('queued')
  })

  it('should show optimistic jobs before base jobs', () => {
    const store = useJobStore()
    store.jobs = [
      { job_id: 'job1', status: 'running' } as any,
      { job_id: 'job2', status: 'done' } as any
    ]

    store.addOptimisticJob({
      job_id: 'job3',
      status: 'queued'
    } as any)

    expect(store.displayJobs).toHaveLength(3)
    expect(store.displayJobs[0].job_id).toBe('job3')
    expect(store.displayJobs[1].job_id).toBe('job1')
    expect(store.displayJobs[2].job_id).toBe('job2')
  })
})
