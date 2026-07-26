export type WorkerRpcResponse = {
  id: number
  ok: boolean
  error?: string
}

type PendingRequest<T> = {
  resolve: (value: T) => void
  reject: (reason?: unknown) => void
  timeoutId: ReturnType<typeof setTimeout> | null
}

export type WorkerRpcRequestOptions = {
  transfer?: Transferable[]
  timeoutMs?: number
}

export class WorkerRpcClient<TRequest extends object, TResponse extends WorkerRpcResponse> {
  private worker: Worker | null = null
  private nextRequestId = 1
  private readonly pending = new Map<number, PendingRequest<TResponse>>()

  constructor(
    private readonly createWorker: () => Worker,
    private readonly label: string,
    private readonly defaultTimeoutMs = 60_000,
  ) {}

  request(payload: TRequest, options: WorkerRpcRequestOptions = {}): Promise<TResponse> {
    const id = this.nextRequestId++
    return new Promise((resolve, reject) => {
      const timeoutMs = options.timeoutMs ?? this.defaultTimeoutMs
      const pending: PendingRequest<TResponse> = {
        resolve,
        reject,
        timeoutId: null,
      }
      if (timeoutMs > 0) {
        pending.timeoutId = setTimeout(() => {
          if (this.pending.get(id) !== pending) return
          this.pending.delete(id)
          reject(new Error(`${this.label} 请求超时`))
        }, timeoutMs)
      }
      this.pending.set(id, pending)
      try {
        this.getWorker().postMessage({ id, ...payload }, options.transfer ?? [])
      } catch (error) {
        this.rejectPending(id, error)
      }
    })
  }

  dispose(reason = `${this.label} 已停止`) {
    const error = new Error(reason)
    for (const pending of this.pending.values()) {
      this.clearPendingTimeout(pending)
      pending.reject(error)
    }
    this.pending.clear()
    this.worker?.terminate()
    this.worker = null
  }

  private getWorker(): Worker {
    if (this.worker) return this.worker
    const worker = this.createWorker()
    worker.onmessage = (event: MessageEvent<TResponse>) => {
      const response = event.data
      const pending = this.pending.get(response.id)
      if (!pending) return
      this.pending.delete(response.id)
      this.clearPendingTimeout(pending)
      pending.resolve(response)
    }
    worker.onerror = (event) => {
      this.dispose(event.message || `${this.label} 发生错误`)
    }
    this.worker = worker
    return worker
  }

  private rejectPending(id: number, reason: unknown) {
    const pending = this.pending.get(id)
    if (!pending) return
    this.pending.delete(id)
    this.clearPendingTimeout(pending)
    pending.reject(reason)
  }

  private clearPendingTimeout(pending: PendingRequest<TResponse>) {
    if (pending.timeoutId !== null) clearTimeout(pending.timeoutId)
  }
}
