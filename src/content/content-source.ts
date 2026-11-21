import { MESSAGE_KINDS, type Message, type MessageKind } from "../domain/types";
import type { Content } from "./schema";

/** Decides WHAT Yub says. Swap in a remote or AI source by implementing this. */
export interface ContentSource {
  next(kind: MessageKind): Message;
}

export const FALLBACK_LINE = "You're doing better than you think.";

interface Options {
  /** How many recent messages per kind to avoid repeating. */
  historySize?: number;
  random?: () => number;
}

export class LocalContentSource implements ContentSource {
  private readonly recent = new Map<MessageKind, string[]>();
  private readonly historySize: number;
  private readonly random: () => number;

  constructor(
    private readonly content: Content,
    { historySize = 3, random = Math.random }: Options = {},
  ) {
    this.historySize = historySize;
    this.random = random;
  }

  next(kind: MessageKind): Message {
    const order = [kind, ...MESSAGE_KINDS.filter((k) => k !== kind)];
    const usable = order.find((k) => this.messagesFor(k).length > 0);
    if (!usable) return { kind, text: FALLBACK_LINE };
    return this.pick(usable, this.messagesFor(usable));
  }

  private messagesFor(kind: MessageKind): Message[] {
    switch (kind) {
      case "goal":
        return this.content.goals.map((g) => ({ kind, text: `Big goal: ${g.title}` }));
      case "quote":
        return this.content.quotes.map((q) => ({ kind, ...q }));
      case "gratitude":
        return this.content.gratitude.map((text) => ({ kind, text: `Grateful for: ${text}` }));
      case "proud":
        return this.content.proud.map((text) => ({ kind, text }));
    }
  }

  private pick(kind: MessageKind, all: Message[]): Message {
    const recent = this.recent.get(kind) ?? [];
    // Keep at least one candidate even if history covers everything.
    const fresh = all.filter((m) => !recent.includes(m.text));
    const pool = fresh.length > 0 ? fresh : all;
    const chosen = pool[Math.floor(this.random() * pool.length)] ?? pool[0]!;
    const keep = Math.min(this.historySize, all.length - 1);
    this.recent.set(kind, keep > 0 ? [...recent, chosen.text].slice(-keep) : []);
    return chosen;
  }
}
