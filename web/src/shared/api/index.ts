import { apiClient } from './client'
import type { JobSummary, LogEntry, PriceBar, DataSource, DataSet } from './types'

export const api = {
  async getJobs(): Promise<JobSummary[]> {
    const { data } = await apiClient.get('/jobs')
    return data
  },

  async stopJob(jobId: string, reason?: string): Promise<void> {
    await apiClient.post(`/jobs/${jobId}/stop`, reason ? { reason } : {})
  },

  async retryJob(jobId: string): Promise<{ job_id: string }> {
    const { data } = await apiClient.post(`/jobs/${jobId}/retry`)
    return data
  },

  async getJobLogs(jobId: string): Promise<LogEntry[]> {
    const { data } = await apiClient.get(`/jobs/${jobId}/logs`)
    return data
  },

  async getKline(symbol: string, timeframe: string): Promise<PriceBar[]> {
    const { data } = await apiClient.get('/api/market/kline', {
      params: { symbol, timeframe }
    })
    return data
  },

  async getHealth(): Promise<{ status: string; mode?: string; last_error?: string }> {
    const { data } = await apiClient.get('/system/health')
    return data
  },

  async getDataSources(): Promise<DataSource[]> {
    const { data } = await apiClient.get('/api/datasources')
    return data
  },

  async getDataSets(): Promise<DataSet[]> {
    const { data } = await apiClient.get('/api/datasets')
    return data
  }
}
