export default {
  recommendationInsights: {
    title: "Recommendation insights",
    description:
      "Only behavior after opening a book from Random picks is counted, so you can see whether recommendations fit your reading choices.",
    period: "{days} days",
    aria: {
      period: "Metric period",
      recommendationRate: "{label} effective reading rate {rate}",
    },
    reload: {
      recommendations: "Reload recommendation data",
      rules: "Reload preference rules",
    },
    error: {
      recommendations: "Recommendation data is temporarily unavailable.",
      rules: "Preference rules are temporarily unavailable.",
    },
    empty: {
      recommendations:
        "No data to show yet. Open and read a comic from Random picks to see recommendation and exploration performance here.",
      rules:
        "The system is still collecting reading signals and has not formed a preference rule to show yet.",
    },
    metrics: {
      effectiveReadRate: "Effective reading rate",
      effectiveReadSummary: "{effectiveReads} effective reads / {opened} opens",
      opened: "Opened",
      exposed: "{count} shown",
      quickExits: "Quick exits",
      quickExitDetail: "Left the reader soon after opening",
      manualDeletesPer100Opens: "Manual deletes per 100 opens",
      manualDeleteDetail: "{count} manual deletes",
      sampleWarning:
        "There are only {count} opens. This data is for observation and is not enough to judge recommendation quality yet.",
    },
    comparison: {
      title: "Preference and exploration",
      detail: "Compare by effective reading rate",
      opened: "{count} opens",
      preferred: "Preferred content",
      exploration: "Exploration content",
    },
    topics: {
      exposed: "Topics reached",
      exposedDetail: "Different topics shown among candidate content",
      exploration: "New topics from exploration",
      explorationDetail: "Topics from unknown or new combinations",
    },
    howItWorks: {
      title: "How recommendations work",
      preferredTitle: "Preferred content",
      preferredDescription:
        "Comic tags and reading behavior give content that is more likely to fit you more opportunities to appear.",
      explorationTitle: "Exploration content",
      explorationDescription:
        "Random picks keep some unknown content in the mix so recommendations do not become too narrow.",
      cleanupTitle: "Automatic cleanup",
      cleanupDescription:
        "Content moves to the recycle bin only when both tag evidence and preference rules reach high confidence; restoring it becomes a correction signal.",
    },
    rules: {
      title: "Preference rules",
      description:
        "Rules you create or the system learns affect future Random picks; system rules are read-only.",
      threshold: "Confidence threshold {value}",
      learnedName: "Learned preference: {condition}",
      pause: "Pause this rule",
      enable: "Enable this rule",
    },
    actions: {
      keep: "Recommend more",
      downrank: "Recommend less",
      autoDelete: "Automatic cleanup",
    },
    statuses: {
      autoPaused: "Auto-paused",
      enabled: "Active",
      disabled: "Not enabled",
    },
    conditions: {
      all: " and ",
      any: " or ",
      combination: "Combined conditions",
      anyCondition: "Any condition",
      not: "Does not include {condition}",
      content: "Content condition",
      confidence: " (≥{value})",
      matches: "{feature}",
      equals: "{feature} = {value}",
      between: "{feature}: {min} - {max}",
      unknownFeature: "Content feature {feature}",
      tag: "Tag {name}",
      theme: "Theme {id}",
      features: {
        page_count: "Page count",
        color_fraction: "Color page ratio",
        gray_fraction: "Grayscale page ratio",
        average_chroma: "Average chroma",
        page_similarity_p50: "Page similarity (median)",
        page_similarity_p90: "Page similarity (P90)",
        duplicate_page_ratio: "Duplicate page ratio",
        visual_change_score: "Visual change",
        layout_stability_score: "Layout stability",
        section_boundary_score: "Section boundary ratio",
        page_aspect_ratio_mean: "Page aspect ratio",
        page_aspect_ratio_variance: "Page aspect ratio variance",
      },
    },
  },
} as const;
