import { useEffect, useMemo, useState } from 'react';
import { Cpu, HardDrive, Plus, Power, Server, Sparkles } from 'lucide-react';
import type { KanbanBoardState, KanbanTask, KanbanTaskStatus } from '@opencode-vibe/protocol';
import { KanbanBoard } from '../components/kanban/KanbanBoard';
import { getConsoleLexiconSection } from '../lexicon/consoleLexicon';

const INITIAL_BOARD: KanbanBoardState = {
  tasks: {
    'demo-1': {
      id: 'demo-1',
      title: '重构任务编排引擎的执行栈',
      status: 'todo',
      description: '拆分状态机，提升跨 Agent 切换稳定性。',
      createdAt: Date.now() - 1000 * 60 * 18,
    },
    'demo-2': {
      id: 'demo-2',
      title: '优化 Gateway 心跳与断线回收',
      status: 'todo',
      description: '减少僵尸会话，新增 60s 失联告警。',
      createdAt: Date.now() - 1000 * 60 * 12,
    },
    'demo-3': {
      id: 'demo-3',
      title: '实现命令流事件压缩',
      status: 'doing',
      description: '降低日志噪声，突出关键动作轨迹。',
      createdAt: Date.now() - 1000 * 60 * 34,
    },
    'demo-4': {
      id: 'demo-4',
      title: '交付看板权限审计报表',
      status: 'done',
      description: '已输出角色矩阵与审计清单。',
      createdAt: Date.now() - 1000 * 60 * 61,
    },
  },
  columns: {
    todo: { id: 'todo', title: 'To Do', taskIds: ['demo-1', 'demo-2'] },
    doing: { id: 'doing', title: 'Doing', taskIds: ['demo-3'] },
    done: { id: 'done', title: 'Done', taskIds: ['demo-4'] },
  },
  columnOrder: ['todo', 'doing', 'done'],
};

const MOCK_TITLES = [
  '新增执行观测热图',
  '整理任务归档压缩策略',
  '强化 Agent 失败重试边界',
  '设计跨项目调度仪表',
];

const SKIN_STORAGE_KEY = 'vk-console-skin';

const readStoredSkin = (): 'neural' | 'lab' => {
  if (typeof window === 'undefined') {
    return 'neural';
  }

  return window.localStorage.getItem(SKIN_STORAGE_KEY) === 'lab' ? 'lab' : 'neural';
};

export function NeuralUiDemo() {
  const [board, setBoard] = useState<KanbanBoardState>(INITIAL_BOARD);
  const [taskCursor, setTaskCursor] = useState(0);
  const [selectedTask, setSelectedTask] = useState<KanbanTask | null>(null);
  const [reactorOnline, setReactorOnline] = useState(true);
  const [skin, setSkin] = useState<'neural' | 'lab'>(readStoredSkin);
  const sharedCopy = getConsoleLexiconSection('shared');
  const demoCopy = getConsoleLexiconSection('demo');

  const taskCount = Object.keys(board.tasks).length;
  const isLabSkin = skin === 'lab';

  useEffect(() => {
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(SKIN_STORAGE_KEY, skin);
    }
  }, [skin]);

  const executingTaskIds = useMemo(
    () => Object.values(board.tasks).filter((task) => task.status === 'doing').map((task) => task.id),
    [board.tasks],
  );

  const moveTask = (taskId: string, targetStatus: KanbanTaskStatus, targetIndex?: number) => {
    setBoard((prev) => {
      const task = prev.tasks[taskId];
      if (!task) return prev;

      const sourceStatus = task.status;
      const nextColumns = {
        ...prev.columns,
        [sourceStatus]: {
          ...prev.columns[sourceStatus],
          taskIds: prev.columns[sourceStatus].taskIds.filter((id) => id !== taskId),
        },
      };

      const nextTargetIds = [...nextColumns[targetStatus].taskIds];
      const insertAt = targetIndex === undefined
        ? nextTargetIds.length
        : Math.max(0, Math.min(targetIndex, nextTargetIds.length));
      nextTargetIds.splice(insertAt, 0, taskId);

      nextColumns[targetStatus] = {
        ...nextColumns[targetStatus],
        taskIds: nextTargetIds,
      };

      return {
        ...prev,
        tasks: {
          ...prev.tasks,
          [taskId]: {
            ...task,
            status: targetStatus,
            updatedAt: Date.now(),
          },
        },
        columns: nextColumns,
      };
    });
  };

  const deleteTask = (taskId: string) => {
    setBoard((prev) => {
      const task = prev.tasks[taskId];
      if (!task) return prev;

      const { [taskId]: _removed, ...remainingTasks } = prev.tasks;

      return {
        ...prev,
        tasks: remainingTasks,
        columns: {
          ...prev.columns,
          [task.status]: {
            ...prev.columns[task.status],
            taskIds: prev.columns[task.status].taskIds.filter((id) => id !== taskId),
          },
        },
      };
    });

    setSelectedTask((current) => (current?.id === taskId ? null : current));
  };

  const createMockTask = () => {
    const nextTitle = MOCK_TITLES[taskCursor % MOCK_TITLES.length];
    const id = `demo-${Date.now()}`;

    setBoard((prev) => ({
      ...prev,
      tasks: {
        ...prev.tasks,
        [id]: {
          id,
          title: nextTitle,
          status: 'todo',
          description: '由 UI Demo 自动注入，用于测试视觉与交互反馈。',
          createdAt: Date.now(),
        },
      },
      columns: {
        ...prev.columns,
        todo: {
          ...prev.columns.todo,
          taskIds: [id, ...prev.columns.todo.taskIds],
        },
      },
    }));

    setTaskCursor((value) => value + 1);
  };

  return (
    <div className={`console-root ${isLabSkin ? 'console-root--lab' : ''}`} data-testid="neural-ui-demo">
      <div className="console-shell">
        <section className="tech-panel command-panel reveal">
          <div className="command-panel__top">
            <div>
              <p className="tech-kicker inline-flex items-center gap-2">
                <Sparkles size={14} /> {demoCopy.kicker}
              </p>
              <h1 className="tech-title">{demoCopy.title}</h1>
              <p className="tech-subtle">
                {demoCopy.subtitle}
              </p>

              <div className="command-panel__labels" aria-label="demo lexicon">
                <span className="command-chip">{sharedCopy.chips.directive}</span>
                <span className="command-chip">{sharedCopy.chips.telemetry}</span>
                <span className="command-chip">{sharedCopy.chips.controlLoop}</span>
              </div>
            </div>

            <div className={`status-beacon ${reactorOnline ? 'status-beacon--online' : 'status-beacon--offline'}`}>
              <span className="status-beacon__dot" />
              {reactorOnline ? sharedCopy.status.reactorOnline : sharedCopy.status.reactorIdle}
            </div>
          </div>

          <div className="command-panel__actions">
            <button type="button" className="tech-btn tech-btn-primary" onClick={createMockTask}>
              <Plus size={16} /> {demoCopy.actions.injectMockTask}
            </button>
            <button
              type="button"
              className="tech-btn tech-btn-secondary"
              onClick={() => setReactorOnline((value) => !value)}
            >
              <Power size={14} /> {demoCopy.actions.toggleReactor}
            </button>
            <button
              type="button"
              className="tech-btn tech-btn-secondary"
              onClick={() => setSkin((prev) => (prev === 'lab' ? 'neural' : 'lab'))}
            >
              {isLabSkin ? sharedCopy.skin.backToNeural : sharedCopy.skin.switchToLab}
            </button>
          </div>
        </section>

        <section className="tech-panel gateway-panel reveal reveal-1">
          <div className="section-bar">
            <div className="flex items-center gap-2">
              <Server size={16} className="text-cyan-300" />
              <h2 className="section-title">{demoCopy.sections.gatewayTitle}</h2>
            </div>
            <p className="section-note">{demoCopy.sections.gatewayNote}</p>
          </div>

          <div className="gateway-grid">
            <div className="gateway-card">
              <div className="gateway-label">Socket</div>
              <div className="gateway-value gateway-value--mono">ws://demo.local:8080/socket.io</div>
            </div>
            <div className="gateway-card">
              <div className="gateway-label">REST API</div>
              <div className="gateway-value gateway-value--mono">http://demo.local:8081/api</div>
            </div>
            <div className="gateway-card">
              <div className="gateway-label">
                <Cpu size={12} /> Worker
              </div>
              <div className="gateway-value gateway-value--mono">worker-alpha | codex-medium</div>
            </div>
            <div className="gateway-card">
              <div className="gateway-label">
                <HardDrive size={12} /> Data Dir
              </div>
              <div className="gateway-value gateway-value--mono">C:\\demo\\vk-data</div>
            </div>
          </div>
        </section>

        <section className="tech-panel board-panel reveal reveal-2">
          <div className="section-bar">
            <h2 className="section-title">{demoCopy.sections.boardTitle}</h2>
            <p className="section-note">{taskCount} {demoCopy.sections.boardCounterSuffix}</p>
          </div>

          <KanbanBoard
            board={board}
            onMoveTask={moveTask}
            onDeleteTask={deleteTask}
            onTaskClick={setSelectedTask}
            executingTaskIds={executingTaskIds}
          />
        </section>

        {selectedTask && (
          <section className="tech-panel gateway-panel">
            <div className="section-bar">
              <h3 className="section-title">{demoCopy.sections.selectedTitle}</h3>
              <p className="section-note">{demoCopy.sections.selectedNote}</p>
            </div>

            <div className="info-block info-block--accent">
              <p className="info-title">{selectedTask.title}</p>
              <p className="tech-subtle mt-0">{selectedTask.description || 'No detail provided.'}</p>
              <div className="command-panel__actions mt-2">
                <button
                  type="button"
                  className="tech-btn tech-btn-primary"
                  onClick={() => moveTask(selectedTask.id, 'done')}
                >
                  {demoCopy.actions.markDone}
                </button>
                <button type="button" className="tech-btn tech-btn-secondary" onClick={() => setSelectedTask(null)}>
                  {demoCopy.actions.closePanel}
                </button>
              </div>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

export default NeuralUiDemo;
