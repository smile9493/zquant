import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { AgentSession } from '../shared/api/types'

export const useAgentStore = defineStore('agent', () => {
  const currentSession = ref<AgentSession | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  return {
    currentSession,
    loading,
    error
  }
})
