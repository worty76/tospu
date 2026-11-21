const TYPE_MS_PER_CHAR = 22;

/** The speech bubble: types text in, can be swapped, shown or hidden. */
export class Bubble {
  readonly el: HTMLDivElement;
  private readonly textEl: HTMLParagraphElement;
  private readonly authorEl: HTMLParagraphElement;
  private typing: ReturnType<typeof setInterval> | null = null;
  private finishTyping: (() => void) | null = null;

  constructor(private readonly instant: boolean) {
    this.el = document.createElement("div");
    this.el.className = "bubble";
    this.el.setAttribute("role", "status");
    this.textEl = document.createElement("p");
    this.textEl.className = "bubble-text";
    this.authorEl = document.createElement("p");
    this.authorEl.className = "bubble-author";
    this.el.append(this.textEl, this.authorEl);
  }

  /** Resolves once the whole text is visible. */
  say(text: string, author?: string): Promise<void> {
    this.stopTyping();
    this.el.classList.add("visible");
    this.authorEl.textContent = author ? `— ${author}` : "";
    this.el.setAttribute("aria-label", author ? `${text} — ${author}` : text);
    if (this.instant) {
      this.textEl.textContent = text;
      return Promise.resolve();
    }
    const chars = [...text];
    let shown = 0;
    this.textEl.textContent = "";
    return new Promise((resolve) => {
      this.finishTyping = resolve;
      this.typing = setInterval(() => {
        shown += 1;
        this.textEl.textContent = chars.slice(0, shown).join("");
        if (shown >= chars.length) {
          this.stopTyping();
        }
      }, TYPE_MS_PER_CHAR);
    });
  }

  hide(): void {
    this.stopTyping();
    this.el.classList.remove("visible");
  }

  /** Stops typing and settles any pending `say()` so callers never hang. */
  private stopTyping() {
    if (this.typing !== null) clearInterval(this.typing);
    this.typing = null;
    const done = this.finishTyping;
    this.finishTyping = null;
    done?.();
  }
}
