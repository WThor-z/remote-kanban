import WebSocket from 'ws';
import { EventEmitter } from 'events';
import type { 
  GatewayOptions, 
  ConnectionState,
  GatewayToServerMessage,
  ServerToGatewayMessage 
} from './types.js';

export class GatewayConnection extends EventEmitter {
  private ws: WebSocket | null = null;
  private state: ConnectionState = {
    status: 'disconnected',
    reconnectAttempt: 0,
  };
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;

  constructor(private options: GatewayOptions) {
    super();
  }

  async connect(): Promise<void> {
    if (this.state.status !== 'disconnected') {
      throw new Error(`Cannot connect: current status is ${this.state.status}`);
    }

    this.state.status = 'connecting';
    this.emit('stateChange', this.state);

    const url = new URL('/agent/ws', this.options.serverUrl);
    url.searchParams.set('hostId', this.options.hostId);

    try {
      this.ws = new WebSocket(url.toString(), {
        headers: {
          Authorization: `Bearer ${this.options.authToken}`,
        },
      });

      this.ws.on('open', () => this.handleOpen());
      this.ws.on('message', (data) => this.handleMessage(data));
      this.ws.on('close', (code, reason) => this.handleClose(code, reason.toString()));
      this.ws.on('error', (err) => this.handleError(err));

    } catch (err) {
      this.state.status = 'disconnected';
      this.state.lastError = err instanceof Error ? err.message : String(err);
      this.emit('stateChange', this.state);
      throw err;
    }
  }

  private handleOpen(): void {
    console.log('[Gateway] WebSocket connected');
    this.state.status = 'connected';
    this.state.reconnectAttempt = 0;
    this.emit('stateChange', this.state);

    // Send registration message
    this.send({
      type: 'register',
      hostId: this.options.hostId,
      capabilities: this.options.capabilities,
    });

    // Start heartbeat
    this.startHeartbeat();
  }

  private handleMessage(data: WebSocket.RawData): void {
    try {
      const msg: ServerToGatewayMessage = JSON.parse(data.toString());
      this.emit('message', msg);

      if (msg.type === 'registered') {
        if (msg.ok) {
          this.state.status = 'registered';
          this.emit('stateChange', this.state);
          console.log('[Gateway] Registered successfully');
        } else {
          console.error('[Gateway] Registration failed:', msg.error);
          this.disconnect();
        }
      } else if (msg.type === 'ping') {
        this.send({ type: 'heartbeat', timestamp: Date.now() });
      }
    } catch (err) {
      console.error('[Gateway] Failed to parse message:', err);
    }
  }

  private handleClose(code: number, reason: string): void {
    console.log(`[Gateway] WebSocket closed: ${code} ${reason}`);
    this.cleanup();

    if (this.options.reconnect !== false) {
      this.scheduleReconnect();
    }
  }

  private handleError(err: Error): void {
    console.error('[Gateway] WebSocket error:', err.message);
    this.state.lastError = err.message;
    if (this.listenerCount('error') > 0) {
      this.emit('error', err);
    }
  }

  send(msg: GatewayToServerMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    } else {
      console.warn('[Gateway] Cannot send: WebSocket not open');
    }
  }

  disconnect(): void {
    this.options.reconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.cleanup();
  }

  private cleanup(): void {
    this.state.status = 'disconnected';
    this.emit('stateChange', this.state);

    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }

    if (this.ws) {
      this.ws.removeAllListeners();
      if (this.ws.readyState === WebSocket.OPEN) {
        this.ws.close();
      }
      this.ws = null;
    }
  }

  private startHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
    }
    this.heartbeatTimer = setInterval(() => {
      this.send({ type: 'heartbeat', timestamp: Date.now() });
    }, 30000);
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) return;

    const delay = this.getReconnectDelay();
    console.log(`[Gateway] Reconnecting in ${delay}ms...`);

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.state.reconnectAttempt++;
      this.connect().catch((err) => {
        console.error('[Gateway] Reconnect failed:', err.message);
      });
    }, delay);
  }

  private getReconnectDelay(): number {
    const baseDelay = 1000;
    const maxDelay = 60000;
    const delay = Math.min(
      baseDelay * Math.pow(2, this.state.reconnectAttempt),
      maxDelay
    );
    // Add ±25% jitter
    return delay * (0.75 + Math.random() * 0.5);
  }

  get isConnected(): boolean {
    return this.state.status === 'registered';
  }

  get currentState(): ConnectionState {
    return { ...this.state };
  }
}
