import {
  type KanbanBoardState,
  type KanbanTask,
  type KanbanTaskStatus,
  createEmptyBoardState,
  generateTaskId,
} from '@opencode-vibe/protocol';

export type StateChangeCallback = (state: KanbanBoardState) => void;

export class KanbanStore {
  private state: KanbanBoardState;
  private listeners: Set<StateChangeCallback> = new Set();

  constructor(initialState?: KanbanBoardState) {
    this.state = initialState ?? createEmptyBoardState();
  }

  getState(): KanbanBoardState {
    return this.state;
  }

  loadState(newState: KanbanBoardState): void {
    this.state = newState;
    this.notify();
  }

  createTask(title: string, description?: string): KanbanTask {
    if (!title || title.trim() === '') {
      throw new Error('任务标题不能为空');
    }

    const task: KanbanTask = {
      id: generateTaskId(),
      title: title.trim(),
      status: 'todo',
      description,
      createdAt: Date.now(),
    };

    this.state = {
      ...this.state,
      tasks: {
        ...this.state.tasks,
        [task.id]: task,
      },
      columns: {
        ...this.state.columns,
        todo: {
          ...this.state.columns.todo,
          taskIds: [...this.state.columns.todo.taskIds, task.id],
        },
      },
    };

    this.notify();
    return task;
  }

  moveTask(taskId: string, targetStatus: KanbanTaskStatus, targetIndex?: number): void {
    const task = this.state.tasks[taskId];
    if (!task) {
      throw new Error('任务不存在');
    }

    const sourceStatus = task.status;

    // 从原列移除
    const sourceColumn = this.state.columns[sourceStatus];
    const newSourceTaskIds = sourceColumn.taskIds.filter((id: string) => id !== taskId);

    // 添加到目标列
    const targetColumn = this.state.columns[targetStatus];
    let newTargetTaskIds: string[];

    if (sourceStatus === targetStatus) {
      // 同列内移动
      newTargetTaskIds = newSourceTaskIds;
      if (targetIndex !== undefined && targetIndex >= 0) {
        newTargetTaskIds.splice(targetIndex, 0, taskId);
      } else {
        newTargetTaskIds.push(taskId);
      }
    } else {
      // 跨列移动
      newTargetTaskIds = [...targetColumn.taskIds];
      if (targetIndex !== undefined && targetIndex >= 0) {
        newTargetTaskIds.splice(targetIndex, 0, taskId);
      } else {
        newTargetTaskIds.push(taskId);
      }
    }

    // 更新任务状态
    const updatedTask: KanbanTask = { ...task, status: targetStatus };

    this.state = {
      ...this.state,
      tasks: {
        ...this.state.tasks,
        [taskId]: updatedTask,
      },
      columns: {
        ...this.state.columns,
        [sourceStatus]: {
          ...sourceColumn,
          taskIds: sourceStatus === targetStatus ? newTargetTaskIds : newSourceTaskIds,
        },
        ...(sourceStatus !== targetStatus && {
          [targetStatus]: {
            ...targetColumn,
            taskIds: newTargetTaskIds,
          },
        }),
      },
    };

    this.notify();
  }

  deleteTask(taskId: string): void {
    const task = this.state.tasks[taskId];
    if (!task) {
      throw new Error('任务不存在');
    }

    const { [taskId]: removed, ...remainingTasks } = this.state.tasks;
    const column = this.state.columns[task.status];

    this.state = {
      ...this.state,
      tasks: remainingTasks,
      columns: {
        ...this.state.columns,
        [task.status]: {
          ...column,
          taskIds: column.taskIds.filter((id: string) => id !== taskId),
        },
      },
    };

    this.notify();
  }

  subscribe(callback: StateChangeCallback): () => void {
    this.listeners.add(callback);
    return () => {
      this.listeners.delete(callback);
    };
  }

  private notify(): void {
    for (const listener of this.listeners) {
      listener(this.state);
    }
  }
}
