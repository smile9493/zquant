import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { WsClient, type WsMessage } from '../shared/ws'
import { decodeWsMessage } from '../shared/ws/decode'
import { reduceSnapshot, reduceJobEvent, reduceLog } from '../shared/ws/events'
import type { JobSummary, LogEntry } from '../shared/api/types'

interface PendingJobAction {
  type: 'stopping' | 'retrying'
  startedAt: number
  jobId: string
}

export const useJobStore = defineStore('jobs', () => {
  const selectedJobId = ref<string | null>(null)
  const wsConnected = ref(false)
  const jobs = ref<JobSummary[]>([])
  const logs = ref<Map<string, LogEntry[]>>(new Map())
  const pendingActions = ref<Map<string, PendingJobAction>>(new Map())
  const optimisticJobs = ref<Map<string, JobSummary>>(new Map())

  const wsUrl = import.meta.env.VITE_WS_URL || 'ws://localhost:3000/ws'
  const wsClient = new WsClient(wsUrl)

  const selectJob = (jobId: string) => {
    selectedJobId.value = jobId
    if (wsClient.isConnected()) {
      wsClient.send({ type: 'subscribe', data: { job_id: jobId } })
    }
  }

  const clearSelectedJob = () => {
    selectedJobId.value = null
  }

  const handleWsMessage = (msg: WsMessage) => {
    const typed = decodeWsMessage(msg)
    if (!typed) return

    if (typed.type === 'snapshot') {
      jobs.value = reduceSnapshot(jobs.value, typed)
      const snapshotJobs = typed.data.jobs || []
      const snapshotJobIds = new Set(snapshotJobs.map(j => j.job_id))

      for (const [jobId, pending] of pendingActions.value.entries()) {
        const snapshotJob = snapshotJobs.find(j => j.job_id === jobId)
        if (!snapshotJob) continue

        if (pending.type === 'stopping' && snapshotJob.stop_requested) {
          pendingActions.value.delete(jobId)
        }
      }

      for (const jobId of optimisticJobs.value.keys()) {
        if (snapshotJobIds.has(jobId)) {
          optimisticJobs.value.delete(jobId)
        }
      }
    } else if (typed.type === 'event') {
      jobs.value = reduceJobEvent(jobs.value, typed, typed.ts)
      if (typed.data.payload.job_id) {
        pendingActions.value.delete(typed.data.payload.job_id)
        optimisticJobs.value.delete(typed.data.payload.job_id)
      }
    } else if (typed.type === 'log') {
      logs.value = reduceLog(logs.value, typed)
    }
  }

  const handleStateChange = (connected: boolean) => {
    wsConnected.value = connected
    if (connected && selectedJobId.value) {
      wsClient.send({ type: 'subscribe', data: { job_id: selectedJobId.value } })
    }
  }

  const initWs = () => {
    wsClient.connect()
    const unsubscribeMsg = wsClient.onMessage(handleWsMessage)
    const unsubscribeState = wsClient.onStateChange(handleStateChange)
    return () => {
      unsubscribeMsg()
      unsubscribeState()
    }
  }

  const disconnectWs = () => {
    wsClient.disconnect()
    wsConnected.value = false
  }

  const displayJobs = computed(() => {
    const baseJobs = jobs.value.map(job => {
      const pending = pendingActions.value.get(job.job_id)
      if (!pending) return job

      if (pending.type === 'stopping') {
        return { ...job, stop_requested: true }
      }
      if (pending.type === 'retrying') {
        return { ...job, _retrying: true }
      }
      return job
    })

    const optimisticList = Array.from(optimisticJobs.value.values())
    return [...optimisticList, ...baseJobs]
  })

  const applyOptimisticStop = (jobId: string) => {
    pendingActions.value.set(jobId, {
      type: 'stopping',
      startedAt: Date.now(),
      jobId
    })
  }

  const applyOptimisticRetry = (jobId: string) => {
    pendingActions.value.set(jobId, {
      type: 'retrying',
      startedAt: Date.now(),
      jobId
    })
  }

  const clearOptimistic = (jobId: string) => {
    pendingActions.value.delete(jobId)
  }

  const addOptimisticJob = (job: JobSummary) => {
    optimisticJobs.value.set(job.job_id, job)
  }

  const setJobs = (newJobs: JobSummary[]) => {
    jobs.value = newJobs
  }

  return {
    selectedJobId,
    wsConnected,
    jobs,
    displayJobs,
    logs,
    pendingActions,
    selectJob,
    clearSelectedJob,
    initWs,
    disconnectWs,
    applyOptimisticStop,
    applyOptimisticRetry,
    clearOptimistic,
    addOptimisticJob,
    setJobs
  }
})

