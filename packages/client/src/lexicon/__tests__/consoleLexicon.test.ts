import { describe, expect, it } from 'vitest';
import { CONSOLE_LEXICON, getConsoleLexiconSection } from '../consoleLexicon';

describe('console lexicon', () => {
  it('exposes command chips and skin switch copy centrally', () => {
    const shared = getConsoleLexiconSection('shared', 'en');
    const sharedZh = getConsoleLexiconSection('shared', 'zh');

    expect(shared.chips.directive).toBe('Directive');
    expect(sharedZh.chips.directive).toBe('指令流');
    expect(shared.skin.switchToLab).toBe('Switch To Lab-Light');
  });

  it('provides app and demo copy from one source', () => {
    const app = getConsoleLexiconSection('app', 'en');
    const demo = getConsoleLexiconSection('demo', 'en');
    const appZh = getConsoleLexiconSection('app', 'zh');

    expect(app.title).toBe('OpenCode Vibe Command Nexus');
    expect(appZh.title).toBe('OpenCode Vibe 指挥中枢');
    expect(demo.title).toBe('Neural UI Demo');
    expect(demo.actions.injectMockTask).toBe('Inject Mock Task');
  });

  it('defines component-level copy for modals and execution panels', () => {
    expect(CONSOLE_LEXICON.createTaskModal.title).toBe('Create Task Capsule');
    expect(CONSOLE_LEXICON.taskDetailPanel.actions.execute).toBe('Execute Dispatch');
    expect(CONSOLE_LEXICON.executionLogPanel.header).toBe('Execution Stream');
    expect(CONSOLE_LEXICON.runHistoryPanel.labels.eventFilterLabel).toBe('Event');
  });
});
