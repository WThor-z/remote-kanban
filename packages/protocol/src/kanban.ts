export type KanbanTaskStatus = 'todo' | 'doing' | 'done';

export interface KanbanTask {
  id: string;
  title: string;
  status: KanbanTaskStatus;
  description?: string;
  createdAt: number;
  updatedAt?: number;
  /** 关联的 Agent 会话 ID */
  sessionId?: string;
}

export interface KanbanColumn {
  id: KanbanTaskStatus;
  title: string;
  taskIds: string[];
}

export interface KanbanBoardState {
  tasks: Record<string, KanbanTask>;
  columns: Record<KanbanTaskStatus, KanbanColumn>;
  columnOrder: KanbanTaskStatus[];
}

export type KanbanEventType = 'kanban:sync' | 'kanban:create' | 'kanban:move' | 'kanban:delete';

export type KanbanEvent = 
  | { type: 'kanban:sync'; payload: KanbanBoardState }
  | { type: 'kanban:create'; payload: { title: string; description?: string } }
  | { type: 'kanban:move'; payload: { taskId: string; targetStatus: KanbanTaskStatus; targetIndex?: number } }
  | { type: 'kanban:delete'; payload: { taskId: string } };

// Type guards
export const isKanbanEventType = (value: string): value is KanbanEventType => {
  return value === 'kanban:sync' || value === 'kanban:create' || 
         value === 'kanban:move' || value === 'kanban:delete';
};

export const isKanbanTaskStatus = (value: string): value is KanbanTaskStatus => {
  return value === 'todo' || value === 'doing' || value === 'done';
};

// Parse raw JSON into a KanbanEvent (or null if invalid)
export const parseKanbanEvent = (raw: string): KanbanEvent | null => {
  if (!raw.startsWith('{') || !raw.endsWith('}')) {
    return null;
  }

  try {
    const parsed = JSON.parse(raw) as { type?: string; payload?: unknown };

    if (!parsed.type || !isKanbanEventType(parsed.type)) {
      return null;
    }

    const { type, payload } = parsed;

    switch (type) {
      case 'kanban:sync':
        if (isValidBoardState(payload)) {
          return { type, payload };
        }
        break;
      case 'kanban:create':
        if (isValidCreatePayload(payload)) {
          return { type, payload };
        }
        break;
      case 'kanban:move':
        if (isValidMovePayload(payload)) {
          return { type, payload };
        }
        break;
      case 'kanban:delete':
        if (isValidDeletePayload(payload)) {
          return { type, payload };
        }
        break;
    }

    return null;
  } catch {
    return null;
  }
};

// Validation helpers
const isValidBoardState = (payload: unknown): payload is KanbanBoardState => {
  if (!payload || typeof payload !== 'object') return false;
  const p = payload as Record<string, unknown>;
  return (
    typeof p.tasks === 'object' &&
    typeof p.columns === 'object' &&
    Array.isArray(p.columnOrder)
  );
};

const isValidCreatePayload = (payload: unknown): payload is { title: string; description?: string } => {
  if (!payload || typeof payload !== 'object') return false;
  const p = payload as Record<string, unknown>;
  return typeof p.title === 'string' && p.title.length > 0;
};

const isValidMovePayload = (payload: unknown): payload is { taskId: string; targetStatus: KanbanTaskStatus; targetIndex?: number } => {
  if (!payload || typeof payload !== 'object') return false;
  const p = payload as Record<string, unknown>;
  return (
    typeof p.taskId === 'string' &&
    typeof p.targetStatus === 'string' &&
    isKanbanTaskStatus(p.targetStatus)
  );
};

const isValidDeletePayload = (payload: unknown): payload is { taskId: string } => {
  if (!payload || typeof payload !== 'object') return false;
  const p = payload as Record<string, unknown>;
  return typeof p.taskId === 'string';
};

// Factory to create an empty board state
export const createEmptyBoardState = (): KanbanBoardState => ({
  tasks: {},
  columns: {
    todo: { id: 'todo', title: 'To Do', taskIds: [] },
    doing: { id: 'doing', title: 'Doing', taskIds: [] },
    done: { id: 'done', title: 'Done', taskIds: [] },
  },
  columnOrder: ['todo', 'doing', 'done'],
});

// Generate unique task ID
export const generateTaskId = (): string => {
  return `task-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
};
