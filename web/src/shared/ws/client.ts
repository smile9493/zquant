export interface WsMessage {
  v: number
  type: string
  ts: string
  data: any
}

export type WsEventHandler = (msg: WsMessage) => void
export type WsStateHandler = (connected: boolean) => void

export class WsClient {
  private ws: WebSocket | null = null
  private url: string
  private handlers: Set<WsEventHandler> = new Set()
  private stateHandlers: Set<WsStateHandler> = new Set()
  private reconnectTimer: number | null = null
  private reconnectDelay = 1000
  private maxReconnectDelay = 30000
  private manualClose = false

  constructor(url: string) {
    this.url = url
  }

  connect() {
    if (this.ws?.readyState === WebSocket.OPEN) return
    this.manualClose = false

    try {
      this.ws = new WebSocket(this.url)

      this.ws.onopen = () => {
        this.reconnectDelay = 1000
        this.stateHandlers.forEach(h => h(true))
      }

      this.ws.onmessage = (event) => {
        try {
          const msg: WsMessage = JSON.parse(event.data)
          this.handlers.forEach(h => h(msg))
        } catch (e) {
          // Ignore parse errors
        }
      }

      this.ws.onerror = () => {
        // Connection error, will retry
      }

      this.ws.onclose = () => {
        this.stateHandlers.forEach(h => h(false))
        if (!this.manualClose) {
          this.scheduleReconnect()
        }
      }
    } catch (e) {
      this.scheduleReconnect()
    }
  }

  private scheduleReconnect() {
    if (this.reconnectTimer) return

    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null
      this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay)
      this.connect()
    }, this.reconnectDelay)
  }

  disconnect() {
    this.manualClose = true
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
  }

  onMessage(handler: WsEventHandler) {
    this.handlers.add(handler)
    return () => this.handlers.delete(handler)
  }

  onStateChange(handler: WsStateHandler) {
    this.stateHandlers.add(handler)
    return () => this.stateHandlers.delete(handler)
  }

  isConnected() {
    return this.ws?.readyState === WebSocket.OPEN
  }

  send(data: any) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data))
    }
  }
}
