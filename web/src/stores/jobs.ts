import { defineStore } from 'pinia'
import { ref } from 'vue'
import { WsClient, type WsMessage } from '../shared/ws'

export interface JobSummary {
  job_id: string
  job_type: string
  status: string
  stop_requested: boolean
  created_at: string
  updated_at: string
}

export interface LogEntry {
  timestamp: string
  level: string
  message: string
}

export const useJobStore = defineStore('jobs', () => {
  const selectedJobId = ref<string | null>(null)
  const wsConnected = ref(false)
  const jobs = ref<JobSummary[]>([])
  const logs = ref<Map<string, LogEntry[]>>(new Map())

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
    if (msg.type === 'snapshot') {
      if (msg.data.jobs) {
        jobs.value = msg.data.jobs
      }
    } else if (msg.type === 'event') {
      const kind = msg.data.kind
      if (kind === 'job.created' || kind === 'job.started' || kind === 'job.completed') {
        const payload = msg.data.payload
        const idx = jobs.value.findIndex(j => j.job_id === payload.job_id)
        if (idx >= 0) {
          jobs.value[idx] = { ...jobs.value[idx], ...payload, updated_at: msg.ts }
        } else if (kind === 'job.created') {
          jobs.value.unshift(payload)
        }
      }
    } else if (msg.type === 'log') {
      const jobId = msg.data.job_id
      const entry = msg.data.entry
      if (!logs.value.has(jobId)) {
        logs.value.set(jobId, [])
      }
      logs.value.get(jobId)!.push(entry)
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

  return {
    selectedJobId,
    wsConnected,
    jobs,
    logs,
    selectJob,
    clearSelectedJob,
    initWs,
    disconnectWs
  }
})

