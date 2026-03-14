import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useDataSourceStore = defineStore('datasource', () => {
  const selectedDataSourceId = ref<string | null>(null)
  const selectedDataSetId = ref<string | null>(null)

  const selectDataSource = (id: string) => {
    selectedDataSourceId.value = id
  }

  const selectDataSet = (id: string) => {
    selectedDataSetId.value = id
  }

  const clearSelection = () => {
    selectedDataSourceId.value = null
    selectedDataSetId.value = null
  }

  return {
    selectedDataSourceId,
    selectedDataSetId,
    selectDataSource,
    selectDataSet,
    clearSelection
  }
})
