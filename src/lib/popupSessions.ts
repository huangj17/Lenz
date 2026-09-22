/** Orders native events against the asynchronous reservation of a manual request. */
export class PopupSessions {
  private floor = 0;
  private ticket = 0;
  private pending = false;
  private blocked = false;
  private buffered: { id: number; apply: () => void } | undefined;

  begin(): number {
    this.pending = true;
    this.buffered = undefined;
    return ++this.ticket;
  }

  commit(ticket: number, id: number): boolean {
    if (ticket !== this.ticket) return false;
    this.floor = Math.max(this.floor, id);
    this.blocked = true;
    this.pending = false;
    const buffered = this.buffered;
    this.buffered = undefined;
    if (buffered) this.event(buffered.id, buffered.apply);
    return true;
  }

  fail(ticket: number): void {
    if (ticket !== this.ticket) return;
    this.pending = false;
    this.buffered = undefined;
  }

  event(id: number, apply: () => void): void {
    if (this.pending) {
      if (!this.buffered || id >= this.buffered.id) this.buffered = { id, apply };
      return;
    }
    if (id < this.floor || (id === this.floor && this.blocked)) return;
    this.floor = id;
    this.blocked = false;
    apply();
  }
}
