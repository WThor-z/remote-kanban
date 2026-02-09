import type { MemoryExtractCandidate, MemoryExtractContext } from './types.js';

const cleanLine = (value: string): string =>
  value
    .replace(/\s+/g, ' ')
    .replace(/^[-*]\s*/, '')
    .replace(/^\d+[.)]\s*/, '')
    .trim();

const splitSentences = (value: string): string[] =>
  value
    .split(/[\n.!?;。！？；]+/g)
    .map(cleanLine)
    .filter((line) => line.length >= 4);

const containsAny = (line: string, needles: string[]): boolean => {
  const lower = line.toLowerCase();
  return needles.some((needle) => lower.includes(needle.toLowerCase()));
};

const matchesAny = (line: string, patterns: RegExp[]): boolean =>
  patterns.some((pattern) => pattern.test(line));

const uniqueByContent = (items: MemoryExtractCandidate[]): MemoryExtractCandidate[] => {
  const seen = new Set<string>();
  const result: MemoryExtractCandidate[] = [];
  for (const item of items) {
    const key = `${item.scope}|${item.kind}|${item.content.toLowerCase()}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    result.push(item);
  }
  return result;
};

const preferenceNeedles = [
  'prefer',
  'preference',
  'style',
  'naming',
  'always',
  'from now on',
  'use chinese',
  'use english',
  '偏好',
  '习惯',
  '风格',
  '命名',
  '请用',
  '统一用',
  '以后都',
  '默认用',
];

const preferencePatterns = [/^(请|请优先|尽量|偏好|我喜欢|我习惯)/, /(prefer|preferably|from now on|always)/i];

const constraintNeedles = [
  'must',
  'cannot',
  "don't",
  'do not',
  'required',
  'forbidden',
  'never',
  '必须',
  '不得',
  '不能',
  '禁止',
  '不要',
  '务必',
  '严禁',
];

const constraintPatterns = [/^(必须|务必|不得|不能|禁止|不要)/, /(must|required|do not|don't|never|forbidden)/i];

const workflowNeedles = [
  'first',
  'then',
  'step',
  'workflow',
  'checklist',
  '流程',
  '步骤',
  '先',
  '再',
  '然后',
  '最后',
];

const workflowPatterns = [/^(先|再|然后|最后)/, /(first|then|next|finally|step\s*\d+)/i];

const factNeedles = [
  'implemented',
  'added',
  'created',
  'fixed',
  'updated',
  'refactored',
  'resolved',
  '完成',
  '已实现',
  '新增',
  '修复',
  '更新',
  '重构',
  '处理了',
];

const factPatterns = [
  /^(已|已经|完成|新增|修复|更新|重构|处理)/,
  /(implemented|added|created|fixed|updated|refactored|resolved)/i,
];

const appendCandidate = (
  candidates: MemoryExtractCandidate[],
  candidate: MemoryExtractCandidate
): void => {
  candidates.push({
    ...candidate,
    content: cleanLine(candidate.content),
  });
};

export const extractRuleCandidates = (ctx: MemoryExtractContext): MemoryExtractCandidate[] => {
  const promptLines = splitSentences(
    `${ctx.taskTitle ?? ''}\n${ctx.taskDescription ?? ''}\n${ctx.taskPrompt}`
  );
  const outputLines = splitSentences(ctx.taskOutput);
  const candidates: MemoryExtractCandidate[] = [];

  for (const line of promptLines) {
    const isPreference =
      containsAny(line, preferenceNeedles) || matchesAny(line, preferencePatterns);
    const isConstraint =
      containsAny(line, constraintNeedles) || matchesAny(line, constraintPatterns);
    const isWorkflow =
      containsAny(line, workflowNeedles) || matchesAny(line, workflowPatterns);

    if (isPreference) {
      appendCandidate(candidates, {
        scope: 'host',
        kind: 'preference',
        content: line,
        tags: ['preference', 'prompt'],
        confidence: 0.78,
        source: 'auto_rule',
      });
    }

    if (isConstraint) {
      appendCandidate(candidates, {
        scope: 'project',
        kind: 'constraint',
        content: line,
        tags: ['constraint', 'prompt'],
        confidence: 0.84,
        source: 'auto_rule',
      });
    }

    if (isWorkflow) {
      appendCandidate(candidates, {
        scope: 'project',
        kind: 'workflow',
        content: line,
        tags: ['workflow', 'prompt'],
        confidence: 0.7,
        source: 'auto_rule',
      });
    }
  }

  for (const line of outputLines.slice(0, 24)) {
    if (containsAny(line, factNeedles) || matchesAny(line, factPatterns)) {
      appendCandidate(candidates, {
        scope: 'project',
        kind: 'fact',
        content: line,
        tags: ['result'],
        confidence: 0.64,
        source: 'auto_rule',
      });
    }
  }

  return uniqueByContent(candidates).slice(0, 8);
};
