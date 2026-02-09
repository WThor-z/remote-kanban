export const CONSOLE_LEXICON = {
  shared: {
    chips: {
      directive: 'Directive 指令流',
      telemetry: 'Telemetry 遥测',
      missionLane: 'Mission Lane 任务通道',
      controlLoop: 'Control Loop 控制回路',
    },
    skin: {
      switchToLab: 'Switch To Lab-Light',
      backToNeural: 'Back To Neural-Dark',
    },
    status: {
      gatewayOnline: 'Gateway Online',
      gatewayOffline: 'Gateway Offline',
      reactorOnline: 'Reactor Online',
      reactorIdle: 'Reactor Idle',
    },
  },

  app: {
    kicker: 'Command Spine · 指挥骨架',
    title: 'OpenCode Vibe Command Nexus',
    subtitle:
      '面向 AI 协同开发的 Mission Control 控制台。你可以在这里统一监控 Gateway Health、Task Constellation、Execution Stream 与 Worktree 状态，形成稳定的调度闭环。',
    actions: {
      createTask: 'New Task Capsule',
      syncTelemetry: 'Sync Telemetry',
    },
    sections: {
      gatewayTitle: 'Gateway Diagnostics',
      gatewayNote: '实时 Telemetry：Socket、REST、Worker Link、Data Vault',
      boardTitle: 'Task Constellation',
      boardCounterSuffix: 'Task Capsules across Mission Lanes',
    },
  },

  demo: {
    kicker: 'Mission Sandbox · 独立样机',
    title: 'Neural UI Demo',
    subtitle:
      '指挥中心演示环境：不依赖后端连接，专门验证视觉基线、交互节奏与信息密度。拖拽卡片时可观察 Execution Flow，快速评审语言系统与配色皮肤切换。',
    actions: {
      injectMockTask: 'Inject Mock Task',
      toggleReactor: 'Toggle Reactor',
      markDone: 'Mark As Done',
      closePanel: 'Close Panel',
    },
    sections: {
      gatewayTitle: 'Gateway Diagnostics',
      gatewayNote: 'Mock Telemetry · 用于视觉、响应式与可读性校验',
      boardTitle: 'Task Constellation',
      boardCounterSuffix: 'tasks in Mission Lanes · 模拟任务通道',
      selectedTitle: 'Selected Task Capsule',
      selectedNote: 'Quick Ops Panel · 快速动作预览',
    },
  },

  createTaskModal: {
    title: 'Create Task Capsule',
    fields: {
      title: 'Directive Title',
      description: 'Mission Brief',
      agent: 'Agent Runtime',
      branch: 'Base Branch / 基线分支',
      workspace: 'Workspace Scope',
      project: 'Project Scope',
      model: 'Model Profile',
    },
    placeholders: {
      title: '输入任务标题 / mission directive...',
      description: '描述执行目标、约束与交付标准...',
      branch: 'main or release/x',
      workspace: 'Select workspace scope',
      workspaceAny: 'All workspaces',
      workspaceAnyHint: 'Show projects from all workspaces',
      project: 'Select project scope',
      modelSearch: 'Search model profiles...',
      modelDefault: 'Default Profile (auto-select)',
      modelDefaultHint: 'Use gateway bound model profile',
    },
    errors: {
      titleRequired: 'Directive title is required',
      projectRequired: 'Project scope is required',
      noWorkspaces: 'No workspace scope registered',
      noWorkspacesAvailable: 'No workspaces available',
      noProjects: 'No project scope registered',
      noProjectsAvailable: 'No projects available',
      noModelMatch: 'No matching model profile',
      noModelFromHost: 'No model profile from this host',
    },
    actions: {
      cancel: 'Abort',
      create: 'Create Capsule',
      dispatch: 'Create & Dispatch',
    },
  },

  taskDetailPanel: {
    statusLabels: {
      idle: 'Idle 待执行',
      starting: 'Booting 启动中',
      running: 'Running 执行中',
      paused: 'Paused 已暂停',
      completed: 'Completed 已完成',
      failed: 'Failed 失败',
      aborted: 'Aborted 已中止',
    },
    blocks: {
      missionBrief: 'Mission Brief 任务说明',
      isolatedWorktree: 'Isolated Worktree 隔离执行环境',
      branch: 'Branch:',
      state: 'State:',
      path: 'Path:',
      noHistory: '点击 "Execute Dispatch" 启动 Agent 执行链路',
    },
    actions: {
      cleanupWorktree: 'Cleanup Worktree',
      execute: 'Execute Dispatch',
      stop: 'Abort Run',
    },
    placeholders: {
      sendDirective: 'Send directive to agent...',
    },
  },

  executionLogPanel: {
    header: 'Execution Stream · 执行流',
    live: 'Live 在线',
    offline: 'Offline 离线',
    empty: 'Awaiting execution events ...',
    inputPlaceholder: 'Type runtime input for agent...',
  },

  runHistoryPanel: {
    titleHistory: 'Run History 执行历史',
    titleTimeline: 'Run Timeline 事件流',
    statuses: {
      initializing: 'Init 初始化',
      creating_worktree: 'Worktree 创建',
      starting: 'Boot 启动',
      running: 'Running 运行',
      paused: 'Paused 暂停',
      completed: 'Completed 完成',
      failed: 'Failed 失败',
      cancelled: 'Cancelled 取消',
      cleaning_up: 'Cleanup 清理',
    },
    labels: {
      loadingRuns: 'Loading runs ...',
      noRuns: 'No execution record yet',
      noPrompt: '(no prompt preview)',
      notStarted: 'not started',
      eventFilterLabel: 'Event 事件',
      agentEventFilterLabel: 'Agent Event',
      loadingEvents: 'Loading events ...',
      noEvents: 'No events yet',
      loadMore: 'Load More',
    },
  },
} as const;

export type ConsoleLexiconSection = keyof typeof CONSOLE_LEXICON;

export const getConsoleLexiconSection = <TSection extends ConsoleLexiconSection>(
  section: TSection,
) => CONSOLE_LEXICON[section];
