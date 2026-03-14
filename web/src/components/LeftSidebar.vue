<template>
  <div class="left-sidebar">
    <div class="sidebar-section">
      <div class="section-title">快速</div>
      <div
        v-for="symbol in quickList"
        :key="symbol"
        class="sidebar-item"
        @click="selectSymbol(symbol)"
      >
        {{ symbol }}
      </div>
    </div>
    <div class="sidebar-section">
      <div class="section-title">收藏</div>
      <div v-if="favorites.length === 0" class="sidebar-empty">无收藏</div>
      <div
        v-for="symbol in favorites"
        :key="symbol"
        class="sidebar-item"
        @click="selectSymbol(symbol)"
      >
        {{ symbol }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useWorkspaceStore } from '../stores/workspace'
import { useWatchlistStore } from '../stores/watchlist'
import { storeToRefs } from 'pinia'

const workspaceStore = useWorkspaceStore()
const watchlistStore = useWatchlistStore()
const { favorites, quickList } = storeToRefs(watchlistStore)

const selectSymbol = (symbol: string) => {
  workspaceStore.symbol = symbol
}
</script>

<style scoped>
.left-sidebar {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 12px 8px;
  overflow-y: auto;
}

.sidebar-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.section-title {
  font-size: 11px;
  color: #757575;
  text-transform: uppercase;
  margin-bottom: 4px;
  padding: 0 4px;
}

.sidebar-item {
  padding: 6px 8px;
  font-size: 12px;
  color: #b0b0b0;
  cursor: pointer;
  border-radius: 3px;
  transition: all 0.2s;
}

.sidebar-item:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #e0e0e0;
}

.sidebar-empty {
  padding: 6px 8px;
  font-size: 11px;
  color: #666;
}
</style>
