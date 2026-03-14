import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useJobStore = defineStore('jobs', () => {
  const selectedJobId = ref<string | null>(null)

  const selectJob = (jobId: string) => {
    selectedJobId.value = jobId
  }

  const clearSelectedJob = () => {
    selectedJobId.value = null
  }

  return {
    selectedJobId,
    selectJob,
    clearSelectedJob
  }
})

