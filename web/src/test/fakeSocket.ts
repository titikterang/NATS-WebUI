import type { SocketLike } from '@/infrastructure/ws'

/** Test double for SocketLike: records instances and lets tests drive open/message/close. */
export class FakeSocket implements SocketLike {
  static all: FakeSocket[] = []
  onopen: SocketLike['onopen'] = null
  onmessage: SocketLike['onmessage'] = null
  onclose: SocketLike['onclose'] = null
  closed = false
  constructor(readonly url: string) {
    FakeSocket.all.push(this)
  }
  close() {
    this.closed = true
    this.onclose?.({})
  }
  open() {
    this.onopen?.({})
  }
  send(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) })
  }
  /** Server-side close (does not mark as closed by the client). */
  drop() {
    this.onclose?.({})
  }
}
