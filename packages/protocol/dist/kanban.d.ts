export type KanbanTaskStatus = 'todo' | 'doing' | 'done';
export interface KanbanTask {
    id: string;
    title: string;
    status: KanbanTaskStatus;
    description?: string;
    createdAt: number;
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
export type KanbanEvent = {
    type: 'kanban:sync';
    payload: KanbanBoardState;
} | {
    type: 'kanban:create';
    payload: {
        title: string;
        description?: string;
    };
} | {
    type: 'kanban:move';
    payload: {
        taskId: string;
        targetStatus: KanbanTaskStatus;
        targetIndex?: number;
    };
} | {
    type: 'kanban:delete';
    payload: {
        taskId: string;
    };
};
export declare const isKanbanEventType: (value: string) => value is KanbanEventType;
export declare const isKanbanTaskStatus: (value: string) => value is KanbanTaskStatus;
export declare const parseKanbanEvent: (raw: string) => KanbanEvent | null;
export declare const createEmptyBoardState: () => KanbanBoardState;
export declare const generateTaskId: () => string;
