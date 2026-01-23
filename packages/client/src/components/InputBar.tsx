import { useState, type FormEvent } from 'react';
import { useOpencode } from '../hooks/useOpencode';

export const InputBar = () => {
  const { write, isConnected } = useOpencode();
  const [value, setValue] = useState('');

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmed = value.trim();
    if (!trimmed || !isConnected) return;

    write(`${trimmed}\r`);
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
        placeholder={isConnected ? 'Type a command to send to Opencode...' : 'Waiting for server connection...'}
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
