export type WorkerRpcResponse = {
  id: number
  ok: boolean
  error?: string
}

type PendingRequest<T> = {
  resolve: (value: T) => void
  reject: (reason?: unknown) => void
}

export class WorkerRpcClient<TRequest extends object, TResponse extends WorkerRpcResponse> {
  private worker: Worker | null = null
  private nextRequestId = 1
  private readonly pending = new Map<number, PendingRequest<TResponse>>()

  constructor(
    private readonly source: URL,
    private readonly label: string,
  ) {}

  request(payload: TRequest): Promise<TResponse> {
    const id = this.nextRequestId++
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject })
      try {
        this.getWorker().postMessage({ id, ...payload })
      } catch (error) {
        this.pending.delete(id)
        reject(error)
      }
    })
  }

  dispose(reason = `${this.label} 已停止`) {
    const error = new Error(reason)
    for (const pending of this.pending.values()) pending.reject(error)
    this.pending.clear()
    this.worker?.terminate()
    this.worker = null
  }

  private getWorker(): Worker {
    if (this.worker) return this.worker
    const worker = new Worker(this.source, { type: 'module' })
    worker.onmessage = (event: MessageEvent<TResponse>) => {
      const response = event.data
      const pending = this.pending.get(response.id)
      if (!pending) return
      this.pending.delete(response.id)
      pending.resolve(response)
    }
    worker.onerror = (event) => {
      this.dispose(event.message || `${this.label} 发生错误`)
    }
    this.worker = worker
    return worker
  }
}
