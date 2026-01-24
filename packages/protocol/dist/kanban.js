"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateTaskId = exports.createEmptyBoardState = exports.parseKanbanEvent = exports.isKanbanTaskStatus = exports.isKanbanEventType = void 0;
// Type guards
const isKanbanEventType = (value) => {
    return value === 'kanban:sync' || value === 'kanban:create' ||
        value === 'kanban:move' || value === 'kanban:delete';
};
exports.isKanbanEventType = isKanbanEventType;
const isKanbanTaskStatus = (value) => {
    return value === 'todo' || value === 'doing' || value === 'done';
};
exports.isKanbanTaskStatus = isKanbanTaskStatus;
// Parse raw JSON into a KanbanEvent (or null if invalid)
const parseKanbanEvent = (raw) => {
    if (!raw.startsWith('{') || !raw.endsWith('}')) {
        return null;
    }
    try {
        const parsed = JSON.parse(raw);
        if (!parsed.type || !(0, exports.isKanbanEventType)(parsed.type)) {
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
    }
    catch {
        return null;
    }
};
exports.parseKanbanEvent = parseKanbanEvent;
// Validation helpers
const isValidBoardState = (payload) => {
    if (!payload || typeof payload !== 'object')
        return false;
    const p = payload;
    return (typeof p.tasks === 'object' &&
        typeof p.columns === 'object' &&
        Array.isArray(p.columnOrder));
};
const isValidCreatePayload = (payload) => {
    if (!payload || typeof payload !== 'object')
        return false;
    const p = payload;
    return typeof p.title === 'string' && p.title.length > 0;
};
const isValidMovePayload = (payload) => {
    if (!payload || typeof payload !== 'object')
        return false;
    const p = payload;
    return (typeof p.taskId === 'string' &&
        typeof p.targetStatus === 'string' &&
        (0, exports.isKanbanTaskStatus)(p.targetStatus));
};
const isValidDeletePayload = (payload) => {
    if (!payload || typeof payload !== 'object')
        return false;
    const p = payload;
    return typeof p.taskId === 'string';
};
// Factory to create an empty board state
const createEmptyBoardState = () => ({
    tasks: {},
    columns: {
        todo: { id: 'todo', title: 'To Do', taskIds: [] },
        doing: { id: 'doing', title: 'Doing', taskIds: [] },
        done: { id: 'done', title: 'Done', taskIds: [] },
    },
    columnOrder: ['todo', 'doing', 'done'],
});
exports.createEmptyBoardState = createEmptyBoardState;
// Generate unique task ID
const generateTaskId = () => {
    return `task-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
};
exports.generateTaskId = generateTaskId;
