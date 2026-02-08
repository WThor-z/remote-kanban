import { describe, expect, it } from 'vitest';
import { CONSOLE_LEXICON, getConsoleLexiconSection } from '../consoleLexicon';

describe('console lexicon', () => {
  it('exposes command chips and skin switch copy centrally', () => {
    const shared = getConsoleLexiconSection('shared');

    expect(shared.chips.directive).toBe('Directive 指令流');
    expect(shared.skin.switchToLab).toBe('Switch To Lab-Light');
  });

  it('provides app and demo copy from one source', () => {
    const app = getConsoleLexiconSection('app');
    const demo = getConsoleLexiconSection('demo');

    expect(app.title).toBe('OpenCode Vibe Command Nexus');
    expect(demo.title).toBe('Neural UI Demo');
    expect(demo.actions.injectMockTask).toBe('Inject Mock Task');
  });

  it('defines component-level copy for modals and execution panels', () => {
    expect(CONSOLE_LEXICON.createTaskModal.title).toBe('Create Task Capsule');
    expect(CONSOLE_LEXICON.taskDetailPanel.actions.execute).toBe('Execute Dispatch');
    expect(CONSOLE_LEXICON.executionLogPanel.header).toBe('Execution Stream · 执行流');
    expect(CONSOLE_LEXICON.runHistoryPanel.labels.eventFilterLabel).toBe('Event 事件');
  });
});
