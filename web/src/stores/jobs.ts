import { defineStore } from 'pinia'
import { ref } from 'vue'
import { WsClient, type WsMessage } from '../shared/ws'
import { decodeWsMessage } from '../shared/ws/decode'
import { reduceSnapshot, reduceJobEvent, reduceLog } from '../shared/ws/events'
import type { JobSummary, LogEntry } from '../shared/api/types'

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
    const typed = decodeWsMessage(msg)
    if (!typed) return

    if (typed.type === 'snapshot') {
      jobs.value = reduceSnapshot(jobs.value, typed)
    } else if (typed.type === 'event') {
      jobs.value = reduceJobEvent(jobs.value, typed, typed.ts)
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

