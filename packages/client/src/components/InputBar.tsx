import { useState, type FormEvent } from 'react';
import { useOpencode } from '../hooks/useOpencode';
import { useKanban } from '../hooks/useKanban';
import { parseCommand } from '../utils/commandParser';

export const InputBar = () => {
  const { write, isConnected } = useOpencode();
  const { createTask, moveTask, deleteTask } = useKanban();
  const [value, setValue] = useState('');

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmed = value.trim();
    if (!trimmed || !isConnected) return;

    // 尝试解析为 Kanban 指令
    const command = parseCommand(trimmed);
    
    if (command) {
      // 拦截 Kanban 指令，不发往 PTY
      switch (command.type) {
        case 'kanban:create':
          createTask(command.payload.title, command.payload.description);
          break;
        case 'kanban:move':
          moveTask(command.payload.taskId, command.payload.targetStatus, undefined);
          break;
        case 'kanban:delete':
          deleteTask(command.payload.taskId);
          break;
      }
    } else {
      // 普通命令，发往 PTY
      write(`${trimmed}\r`);
    }

    setValue('');
  };

  return (
    <form
      data-testid="input-bar-form"
      onSubmit={handleSubmit}
      className="w-full max-w-6xl bg-slate-800/80 border border-slate-700/50 rounded-full px-3 py-2 flex items-center gap-3 shadow-xl"
    >
      <input
        data-testid="input-bar-input"
        type="text"
        value={value}
        onChange={(event) => setValue(event.target.value)}
        placeholder={isConnected ? 'Type a command or /task add <title>...' : 'Waiting for server connection...'}
        className="flex-1 bg-transparent text-slate-100 placeholder:text-slate-500 text-sm focus:outline-none px-3"
      />
      <button
        data-testid="input-bar-submit"
        type="submit"
        disabled={!isConnected || value.trim().length === 0}
        className="rounded-full bg-emerald-500/90 px-4 py-2 text-sm font-semibold text-slate-900 transition disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
      >
        Send
      </button>
    </form>
  );
};
