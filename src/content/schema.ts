import { z } from "zod";
import defaults from "../../defaults/content.json";

const textItem = z
  .union([z.string().min(1), z.object({ text: z.string().min(1), author: z.string().optional() })])
  .transform((v) => (typeof v === "string" ? { text: v } : v));

const contentSchema = z.object({
  goals: z
    .array(
      z.union([
        z.string().min(1).transform((title) => ({ title })),
        z.object({ title: z.string().min(1), progress: z.number().min(0).max(1).optional() }),
      ]),
    )
    .default([]),
  quotes: z.array(textItem).default([]),
  gratitude: z.array(z.string().min(1)).default([]),
  proud: z.array(z.string().min(1)).default([]),
});

export type Content = z.infer<typeof contentSchema>;

/** Written to content.json on first run (by the Rust side, from the same file). */
export const DEFAULT_CONTENT: Content = contentSchema.parse(defaults);

export function parseContent(json: unknown): { content: Content; error?: string } {
  const r = contentSchema.safeParse(json);
  if (r.success) return { content: r.data };
  const issue = r.error.issues[0];
  return { content: DEFAULT_CONTENT, error: `content.json ${issue?.path.join(".")}: ${issue?.message}` };
}
