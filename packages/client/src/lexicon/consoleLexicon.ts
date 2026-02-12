import type { ConsoleLanguage } from '../i18n/consoleLanguage';

const CONSOLE_LEXICON_BY_LANGUAGE = {
  en: {
    shared: {
      chips: {
        directive: 'Directive',
        telemetry: 'Telemetry',
        missionLane: 'Mission Lane',
        controlLoop: 'Control Loop',
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
      kicker: 'Command Spine',
      title: 'OpenCode Vibe Command Nexus',
      subtitle:
        'Mission control console for AI-assisted development. Observe gateway health, task constellation, execution streams, and worktree status in one place.',
      actions: {
        createTask: 'New Task Capsule',
        syncTelemetry: 'Sync Telemetry',
        workspaceScope: 'Workspace Scope',
      },
      sections: {
        gatewayTitle: 'Gateway Diagnostics',
        gatewayNote: 'Live telemetry for socket, REST, worker link, and data vault',
        boardTitle: 'Task Constellation',
        boardCounterSuffix: 'Task Capsules across Mission Lanes',
      },
    },

    demo: {
      kicker: 'Mission Sandbox',
      title: 'Neural UI Demo',
      subtitle:
        'Standalone visual sandbox for validating layout rhythm, hierarchy, and interaction language without backend dependencies.',
      actions: {
        injectMockTask: 'Inject Mock Task',
        toggleReactor: 'Toggle Reactor',
        markDone: 'Mark As Done',
        closePanel: 'Close Panel',
      },
      sections: {
        gatewayTitle: 'Gateway Diagnostics',
        gatewayNote: 'Mock telemetry for visual and responsive checks',
        boardTitle: 'Task Constellation',
        boardCounterSuffix: 'tasks in Mission Lanes',
        selectedTitle: 'Selected Task Capsule',
        selectedNote: 'Quick Ops Panel',
      },
    },

    createTaskModal: {
      title: 'Create Task Capsule',
      fields: {
        title: 'Directive Title',
        description: 'Mission Brief',
        agent: 'Agent Runtime',
        branch: 'Base Branch',
        workspace: 'Workspace Scope',
        project: 'Project Scope',
        model: 'Model Profile',
      },
      placeholders: {
        title: 'Enter task title / mission directive...',
        description: 'Describe objectives, constraints, and acceptance criteria...',
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
        cancel: 'Cancel',
        create: 'Create Capsule',
        dispatch: 'Create & Dispatch',
      },
    },

    taskDetailPanel: {
      tabs: {
        chat: 'Chat',
        logs: 'Logs',
        runs: 'Runs',
      },
      labels: {
        close: 'Close',
      },
      statusLabels: {
        idle: 'Idle',
        starting: 'Booting',
        running: 'Running',
        paused: 'Paused',
        completed: 'Completed',
        failed: 'Failed',
        aborted: 'Aborted',
      },
      blocks: {
        missionBrief: 'Mission Brief',
        isolatedWorktree: 'Isolated Worktree',
        branch: 'Branch:',
        state: 'State:',
        path: 'Path:',
        noHistory: 'Click "Execute Dispatch" to start the agent workflow',
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
      header: 'Execution Stream',
      live: 'Live',
      offline: 'Offline',
      empty: 'Awaiting execution events ...',
      inputPlaceholder: 'Type runtime input for agent...',
    },

    runHistoryPanel: {
      titleHistory: 'Run History',
      titleTimeline: 'Run Timeline',
      statuses: {
        initializing: 'Init',
        creating_worktree: 'Worktree',
        starting: 'Boot',
        running: 'Running',
        paused: 'Paused',
        completed: 'Completed',
        failed: 'Failed',
        cancelled: 'Cancelled',
        cleaning_up: 'Cleanup',
      },
      labels: {
        loadingRuns: 'Loading runs ...',
        noRuns: 'No execution record yet',
        noPrompt: '(no prompt preview)',
        notStarted: 'not started',
        eventFilterLabel: 'Event',
        agentEventFilterLabel: 'Agent Event',
        loadingEvents: 'Loading events ...',
        noEvents: 'No events yet',
        loadMore: 'Load More',
      },
      filters: {
        all: 'All',
        agent: 'Agent',
        status: 'Status',
        sessionStarted: 'Session Started',
        sessionEnded: 'Session Ended',
        progress: 'Progress',
        thinking: 'Thinking',
        command: 'Command',
        fileChange: 'File Change',
        toolCall: 'Tool Call',
        message: 'Message',
        error: 'Error',
        completed: 'Completed',
        rawOutput: 'Raw Output',
      },
      meta: {
        runsCountSuffix: 'Runs',
        backToRunList: 'Back to run list',
        refreshRuns: 'Refresh runs',
        refreshEvents: 'Refresh events',
        durationUnknown: '—',
        agent: 'Agent',
        events: 'Events',
        run: 'Run',
      },
    },
  },
  zh: {
    shared: {
      chips: {
        directive: '指令流',
        telemetry: '遥测',
        missionLane: '任务通道',
        controlLoop: '控制回路',
      },
      skin: {
        switchToLab: '切换到浅色实验皮肤',
        backToNeural: '切换到深色神经皮肤',
      },
      status: {
        gatewayOnline: '网关在线',
        gatewayOffline: '网关离线',
        reactorOnline: '反应器在线',
        reactorIdle: '反应器空闲',
      },
    },

    app: {
      kicker: '指挥骨架',
      title: 'OpenCode Vibe 指挥中枢',
      subtitle:
        '面向 AI 协同开发的控制台。统一监控网关健康、任务看板、执行流与工作树状态，形成稳定调度闭环。',
      actions: {
        createTask: '新建任务胶囊',
        syncTelemetry: '同步遥测',
        workspaceScope: '工作区范围',
      },
      sections: {
        gatewayTitle: '网关诊断',
        gatewayNote: '实时遥测：Socket、REST、Worker 链路与数据目录',
        boardTitle: '任务星图',
        boardCounterSuffix: '个任务胶囊分布在任务通道',
      },
    },

    demo: {
      kicker: '任务沙盒',
      title: '神经 UI 演示',
      subtitle:
        '独立视觉演示环境：无需后端连接，用于验证布局节奏、层级与交互语言。',
      actions: {
        injectMockTask: '注入模拟任务',
        toggleReactor: '切换反应器',
        markDone: '标记完成',
        closePanel: '关闭面板',
      },
      sections: {
        gatewayTitle: '网关诊断',
        gatewayNote: '模拟遥测（用于视觉与响应式校验）',
        boardTitle: '任务星图',
        boardCounterSuffix: '个任务位于任务通道',
        selectedTitle: '当前任务胶囊',
        selectedNote: '快速操作面板',
      },
    },

    createTaskModal: {
      title: '创建任务胶囊',
      fields: {
        title: '指令标题',
        description: '任务说明',
        agent: 'Agent 运行时',
        branch: '基线分支',
        workspace: '工作区范围',
        project: '项目范围',
        model: '模型配置',
      },
      placeholders: {
        title: '输入任务标题 / 指令...',
        description: '描述目标、约束与验收标准...',
        branch: 'main 或 release/x',
        workspace: '选择工作区范围',
        workspaceAny: '全部工作区',
        workspaceAnyHint: '显示所有工作区项目',
        project: '选择项目范围',
        modelSearch: '搜索模型配置...',
        modelDefault: '默认配置（自动选择）',
        modelDefaultHint: '使用主机绑定模型配置',
      },
      errors: {
        titleRequired: '任务标题不能为空',
        projectRequired: '必须选择项目范围',
        noWorkspaces: '未注册工作区范围',
        noWorkspacesAvailable: '当前没有可用工作区',
        noProjects: '未注册项目范围',
        noProjectsAvailable: '当前没有可用项目',
        noModelMatch: '未找到匹配模型配置',
        noModelFromHost: '当前主机没有可用模型配置',
      },
      actions: {
        cancel: '取消',
        create: '创建任务',
        dispatch: '创建并派发',
      },
    },

    taskDetailPanel: {
      tabs: {
        chat: '对话',
        logs: '日志',
        runs: '运行记录',
      },
      labels: {
        close: '关闭',
      },
      statusLabels: {
        idle: '待执行',
        starting: '启动中',
        running: '执行中',
        paused: '已暂停',
        completed: '已完成',
        failed: '失败',
        aborted: '已中止',
      },
      blocks: {
        missionBrief: '任务说明',
        isolatedWorktree: '隔离工作树',
        branch: '分支：',
        state: '状态：',
        path: '路径：',
        noHistory: '点击“执行派发”以启动 Agent 执行链路',
      },
      actions: {
        cleanupWorktree: '清理工作树',
        execute: '执行派发',
        stop: '中止运行',
      },
      placeholders: {
        sendDirective: '向 Agent 发送指令...',
      },
    },

    executionLogPanel: {
      header: '执行流',
      live: '在线',
      offline: '离线',
      empty: '等待执行事件 ...',
      inputPlaceholder: '输入发给 Agent 的运行指令...',
    },

    runHistoryPanel: {
      titleHistory: '运行历史',
      titleTimeline: '运行时间线',
      statuses: {
        initializing: '初始化',
        creating_worktree: '创建工作树',
        starting: '启动',
        running: '运行中',
        paused: '已暂停',
        completed: '已完成',
        failed: '失败',
        cancelled: '已取消',
        cleaning_up: '清理中',
      },
      labels: {
        loadingRuns: '加载运行记录中 ...',
        noRuns: '暂无执行记录',
        noPrompt: '（无提示词预览）',
        notStarted: '未开始',
        eventFilterLabel: '事件',
        agentEventFilterLabel: 'Agent 事件',
        loadingEvents: '加载事件中 ...',
        noEvents: '暂无事件',
        loadMore: '加载更多',
      },
      filters: {
        all: '全部',
        agent: 'Agent',
        status: '状态变更',
        sessionStarted: '会话开始',
        sessionEnded: '会话结束',
        progress: '进度',
        thinking: '思考',
        command: '命令',
        fileChange: '文件变更',
        toolCall: '工具调用',
        message: '消息',
        error: '错误',
        completed: '完成',
        rawOutput: '原始输出',
      },
      meta: {
        runsCountSuffix: '条运行',
        backToRunList: '返回运行列表',
        refreshRuns: '刷新运行记录',
        refreshEvents: '刷新事件',
        durationUnknown: '—',
        agent: 'Agent',
        events: '事件',
        run: '运行',
      },
    },
  },
} as const;

export const CONSOLE_LEXICON = CONSOLE_LEXICON_BY_LANGUAGE.en;

export type ConsoleLexiconSection = keyof typeof CONSOLE_LEXICON;

export const getConsoleLexicon = (language: ConsoleLanguage = 'en') =>
  CONSOLE_LEXICON_BY_LANGUAGE[language];

export const getConsoleLexiconSection = <TSection extends ConsoleLexiconSection>(
  section: TSection,
  language: ConsoleLanguage = 'en',
) => CONSOLE_LEXICON_BY_LANGUAGE[language][section];
