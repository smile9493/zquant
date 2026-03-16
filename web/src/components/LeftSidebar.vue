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
      <div v-if="favorites.length === 0" class="zq-empty-state">无收藏</div>
      <div
        v-for="symbol in favorites"
        :key="symbol"
        class="sidebar-item-row"
      >
        <div class="sidebar-item" @click="selectSymbol(symbol)">
          {{ symbol }}
        </div>
        <button class="remove-btn" @click="removeFav(symbol)" title="移除">×</button>
      </div>
      <div class="add-favorite">
        <input
          v-model="newFavorite"
          @keyup.enter="addFav"
          placeholder="添加..."
          class="add-input"
        />
        <button @click="addFav" class="add-btn">+</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { useWatchlistStore } from '../stores/watchlist'
import { storeToRefs } from 'pinia'

const workspaceStore = useWorkspaceStore()
const watchlistStore = useWatchlistStore()
const { favorites, quickList } = storeToRefs(watchlistStore)
const newFavorite = ref('')

const selectSymbol = (symbol: string) => {
  workspaceStore.symbol = symbol
}

const addFav = () => {
  const symbol = newFavorite.value.trim().toUpperCase()
  if (symbol) {
    watchlistStore.addFavorite(symbol)
    newFavorite.value = ''
  }
}

const removeFav = (symbol: string) => {
  watchlistStore.removeFavorite(symbol)
}
</script>

<style scoped>
.left-sidebar {
  display: flex;
  flex-direction: column;
  gap: var(--zq-space-4);
  padding: var(--zq-space-3) var(--zq-space-2);
  overflow-y: auto;
}

.sidebar-section {
  display: flex;
  flex-direction: column;
  gap: var(--zq-space-1);
}

.section-title {
  font-size: var(--zq-font-size-xs);
  color: var(--zq-text-label);
  text-transform: uppercase;
  margin-bottom: var(--zq-space-1);
  padding: 0 var(--zq-space-1);
}

.sidebar-item {
  padding: var(--zq-space-1) var(--zq-space-2);
  font-size: var(--zq-font-size-sm);
  color: var(--zq-text-secondary);
  cursor: pointer;
  border-radius: var(--zq-radius-sm);
  transition: all var(--zq-transition-base);
  flex: 1;
}

.sidebar-item:hover {
  background: var(--zq-bg-panel-hover);
  color: var(--zq-text-primary);
}

.sidebar-item-row {
  display: flex;
  align-items: center;
  gap: var(--zq-space-1);
}

.remove-btn {
  padding: var(--zq-space-1) var(--zq-space-1);
  background: var(--zq-interactive-danger-bg);
  border: none;
  border-radius: var(--zq-radius-sm);
  color: var(--zq-interactive-danger);
  font-size: var(--zq-font-size-base);
  cursor: pointer;
  opacity: var(--zq-opacity-disabled);
  transition: opacity var(--zq-transition-base);
}

.remove-btn:hover {
  opacity: 1;
}

.add-favorite {
  display: flex;
  gap: var(--zq-space-1);
  margin-top: var(--zq-space-1);
}

.add-input {
  flex: 1;
  padding: var(--zq-space-1) var(--zq-space-1);
  background: var(--zq-bg-input);
  border: 1px solid var(--zq-border-input);
  border-radius: var(--zq-radius-sm);
  color: var(--zq-text-primary);
  font-size: var(--zq-font-size-xs);
}

.add-input:focus {
  outline: none;
  border-color: var(--zq-border-input-focus);
}

.add-btn {
  padding: var(--zq-space-1) var(--zq-space-2);
  background: var(--zq-interactive-primary-bg);
  border: 1px solid var(--zq-interactive-primary-border);
  border-radius: var(--zq-radius-sm);
  color: var(--zq-interactive-primary);
  font-size: var(--zq-font-size-base);
  cursor: pointer;
  transition: all var(--zq-transition-base);
}

.add-btn:hover {
  background: var(--zq-interactive-primary-bg-hover);
}
</style>
