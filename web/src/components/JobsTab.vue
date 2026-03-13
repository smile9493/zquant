<template>
  <div class="jobs-tab">
    <div v-if="isLoading" class="status">Loading...</div>
    <div v-else-if="error" class="status error">Error loading jobs</div>
    <div v-else-if="!data?.length" class="status">No jobs running</div>
    <div v-else class="job-list">
      <div
        v-for="job in data"
        :key="job.id"
        class="job-item"
        @click="selectJob(job.id)"
      >
        <span class="job-id">{{ job.id }}</span>
        <span :class="['job-status', job.status]">{{ job.status }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'
import { api } from '../shared/api'
import { useWorkspaceStore } from '../stores/workspace'

const store = useWorkspaceStore()

const { data, isLoading, error } = useQuery({
  queryKey: ['jobs'],
  queryFn: api.getJobs,
  refetchInterval: 5000
})

const selectJob = (jobId: string) => {
  store.selectedJobId = jobId
}
</script>

<style scoped>
.jobs-tab {
  padding: 16px;
  height: 100%;
  overflow-y: auto;
}

.status {
  padding: 8px;
  color: #b0b0b0;
}

.status.error {
  color: #ef5350;
}

.job-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.job-item {
  display: flex;
  justify-content: space-between;
  padding: 8px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 4px;
}

.job-id {
  color: #e0e0e0;
}

.job-status {
  padding: 2px 8px;
  border-radius: 3px;
  font-size: 12px;
}

.job-status.running {
  background: #1976d2;
}

.job-status.completed {
  background: #388e3c;
}

.job-status.failed {
  background: #d32f2f;
}

.job-status.pending {
  background: #757575;
}
</style>
