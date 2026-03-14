import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useWorkspaceStore = defineStore('workspace', () => {
  const mode = ref<'research'>('research')
  const symbol = ref('AAPL')
  const timeframe = ref('1D')
  const rightPanel = ref('data-explorer')
  const bottomPanel = ref('jobs')

  return {
    mode,
    symbol,
    timeframe,
    rightPanel,
    bottomPanel
  }
})
