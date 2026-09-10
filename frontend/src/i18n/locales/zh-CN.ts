export default {
  recommendationInsights: {
    title: "推荐洞察",
    description:
      "只统计从书库“随机精选”进入阅读器后的行为，用来观察推荐是否更贴近你的阅读选择。",
    period: "{days} 天",
    aria: {
      period: "统计周期",
      recommendationRate: "{label}有效阅读率 {rate}",
    },
    reload: {
      recommendations: "重新加载推荐数据",
      rules: "重新加载偏好规则",
    },
    error: {
      recommendations: "暂时无法读取推荐数据。",
      rules: "暂时无法读取偏好规则。",
    },
    empty: {
      recommendations:
        "暂无可展示的数据。开始从“随机精选”打开并阅读漫画后，这里会显示推荐与探索内容的表现。",
      rules: "系统还在积累阅读信号，尚未形成可展示的偏好规则。",
    },
    metrics: {
      effectiveReadRate: "有效阅读率",
      effectiveReadSummary: "{effectiveReads} 次有效阅读 / {opened} 次打开",
      opened: "已打开",
      exposed: "共展示 {count} 本",
      quickExits: "快速退出",
      quickExitDetail: "打开后较快离开阅读器",
      manualDeletesPer100Opens: "每百次打开的手动删除",
      manualDeleteDetail: "{count} 次手动删除",
      sampleWarning:
        "当前只有 {count} 次打开记录，数据仅供观察，暂不适合据此判断推荐效果。",
    },
    comparison: {
      title: "偏好与探索",
      detail: "按有效阅读率对比",
      opened: "{count} 次打开",
      preferred: "偏好内容",
      exploration: "探索内容",
    },
    topics: {
      exposed: "已接触题材",
      exposedDetail: "候选内容中已展示的不同题材数",
      exploration: "探索带来的新题材",
      explorationDetail: "来自未知或新组合的题材数",
    },
    howItWorks: {
      title: "推荐如何工作",
      preferredTitle: "偏好内容",
      preferredDescription:
        "系统会结合已完成的内容分析与阅读行为，为更可能合适的内容增加出现机会。",
      explorationTitle: "探索内容",
      explorationDescription:
        "随机精选会保留一部分未知内容，避免推荐范围越来越窄。",
      cleanupTitle: "自动清理",
      cleanupDescription:
        "只有内容证据和偏好规则都达到高置信度时才会移入回收站；恢复会作为纠正信号。",
    },
    rules: {
      title: "偏好规则",
      description:
        "你创建或系统学习出的规则会影响后续随机精选；系统级规则只读。",
      threshold: "置信度阈值 {value}",
      learnedName: "系统学习：{condition}",
      pause: "暂停此规则",
      enable: "启用此规则",
    },
    actions: {
      keep: "优先推荐",
      downrank: "降低推荐",
      autoDelete: "自动清理",
    },
    statuses: {
      autoPaused: "已自动暂停",
      enabled: "生效中",
      disabled: "暂未启用",
    },
    conditions: {
      all: " 且 ",
      any: " 或 ",
      combination: "组合条件",
      anyCondition: "任一条件",
      not: "不包含 {condition}",
      content: "内容条件",
      confidence: "（≥{value}）",
      matches: "{feature}",
      equals: "{feature} = {value}",
      between: "{feature}：{min} - {max}",
      unknownFeature: "内容特征 {feature}",
      tag: "标签 {name}",
      theme: "主题 {id}",
      features: {
        page_count: "页数",
        color_fraction: "彩色页面比例",
        gray_fraction: "灰度页面比例",
        average_chroma: "平均色度",
        page_similarity_p50: "页面相似度（中位数）",
        page_similarity_p90: "页面相似度（P90）",
        duplicate_page_ratio: "重复页面比例",
        visual_change_score: "画面变化程度",
        layout_stability_score: "版式稳定度",
        section_boundary_score: "分节边界比例",
        page_aspect_ratio_mean: "页面宽高比",
        page_aspect_ratio_variance: "页面宽高比波动",
      },
    },
  },
} as const;
