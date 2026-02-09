import { z } from 'zod';

import type { MemoryExtractCandidate, MemoryExtractContext } from './types.js';

const candidateSchema = z.object({
  scope: z.enum(['project', 'host']).default('project'),
  kind: z.enum(['preference', 'constraint', 'fact', 'workflow']).default('fact'),
  content: z.string().min(8),
  tags: z.array(z.string()).default([]),
  confidence: z.number().min(0).max(1).default(0.6),
});

const responseSchema = z.array(candidateSchema);

const sleep = async (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

const parseModel = (
  model?: string
): { providerID: string; modelID: string } | undefined => {
  if (!model) {
    return undefined;
  }
  const parts = model.split('/');
  if (parts.length < 2) {
    return undefined;
  }
  return {
    providerID: parts[0],
    modelID: parts.slice(1).join('/'),
  };
};

const extractJsonArray = (raw: string): string | null => {
  const fenced = raw.match(/```json\s*([\s\S]*?)```/i);
  if (fenced && fenced[1]) {
    return fenced[1].trim();
  }
  const start = raw.indexOf('[');
  const end = raw.lastIndexOf(']');
  if (start >= 0 && end > start) {
    return raw.slice(start, end + 1);
  }
  return null;
};

const toCandidates = (raw: string): MemoryExtractCandidate[] => {
  const jsonText = extractJsonArray(raw);
  if (!jsonText) {
    return [];
  }
  const parsed = responseSchema.safeParse(JSON.parse(jsonText));
  if (!parsed.success) {
    return [];
  }
  return parsed.data.map((item) => ({
    ...item,
    source: 'auto_llm' as const,
  }));
};

export const shouldRunLlmFallback = (
  ruleCandidates: MemoryExtractCandidate[],
  minCount = 3,
  minAvgConfidence = 0.65
): boolean => {
  if (ruleCandidates.length < minCount) {
    return true;
  }
  const avg =
    ruleCandidates.reduce((sum, item) => sum + item.confidence, 0) / ruleCandidates.length;
  return avg < minAvgConfidence;
};

export const extractLlmCandidates = async (params: {
  opencodeClient?: any;
  model?: string;
  context: MemoryExtractContext;
}): Promise<MemoryExtractCandidate[]> => {
  const { opencodeClient, model, context } = params;
  if (!opencodeClient?.session) {
    return [];
  }

  const instruction = [
    'Extract durable memory items for future coding tasks.',
    'Return JSON array only.',
    'Each item format:',
    '{"scope":"project|host","kind":"preference|constraint|fact|workflow","content":"...","tags":["..."],"confidence":0.0-1.0}',
    'Keep only stable and reusable information.',
    '',
    `Task title: ${context.taskTitle ?? ''}`,
    `Task description: ${context.taskDescription ?? ''}`,
    `Prompt: ${context.taskPrompt}`,
    `Output: ${context.taskOutput.slice(0, 4000)}`,
  ].join('\n');

  let sessionId: string | null = null;

  try {
    const sessionResult = await opencodeClient.session.create({
      body: { title: 'memory-extract' },
    });
    sessionId = sessionResult?.data?.id ?? null;
    if (!sessionId) {
      return [];
    }

    await opencodeClient.session.promptAsync({
      path: { id: sessionId },
      body: {
        model: parseModel(model),
        parts: [{ type: 'text', text: instruction }],
      },
    });

    const deadline = Date.now() + 25_000;
    while (Date.now() < deadline) {
      const messagesResult = await opencodeClient.session.messages({
        path: { id: sessionId },
      });
      const messages = messagesResult?.data ?? [];
      const assistant = [...messages]
        .reverse()
        .find((msg: any) => msg?.info?.role === 'assistant');
      if (assistant) {
        const parts = assistant.parts ?? [];
        const text = parts
          .filter((part: any) => part?.type === 'text')
          .map((part: any) => String(part.text ?? ''))
          .join('\n');
        const candidates = toCandidates(text);
        if (candidates.length > 0) {
          return candidates.slice(0, 8);
        }
        if (assistant?.info?.time?.completed) {
          return [];
        }
      }
      await sleep(500);
    }
    return [];
  } catch {
    return [];
  } finally {
    if (sessionId) {
      try {
        await opencodeClient.session.abort({ path: { id: sessionId } });
      } catch {
        // Ignore cleanup errors.
      }
    }
  }
};
