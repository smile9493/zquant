import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useWorkspaceStore = defineStore('workspace', () => {
  const symbol = ref('AAPL')
  const timeframe = ref('1D')
  const rightPanel = ref('data-explorer')
  const bottomPanel = ref('jobs')
  const selectedJobId = ref<string | null>(null)

  return {
    symbol,
    timeframe,
    rightPanel,
    bottomPanel,
    selectedJobId
  }
})
