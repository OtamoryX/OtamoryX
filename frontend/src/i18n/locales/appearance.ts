export default {
  "zh-CN": {
    title: "外观设置",
    themeMode: "主题模式",
    themeOptions: {
      light: {
        label: "浅色",
        description: "亮背景，适合白天使用",
      },
      dark: {
        label: "深色",
        description: "暗背景，适合夜间阅读",
      },
      system: {
        label: "跟随系统",
        description: "自动跟随系统主题",
      },
    },
    randomPicks: {
      title: "显示随机精选",
      description: "在书库顶部展示随机精选轮播",
    },
    rowsPerPage: {
      title: "每页显示行数",
      description: "书库列表每页显示行数（列数随屏幕宽度自适应）",
    },
  },
  "en-US": {
    title: "Appearance",
    themeMode: "Theme mode",
    themeOptions: {
      light: {
        label: "Light",
        description: "Bright background for daytime use",
      },
      dark: {
        label: "Dark",
        description: "Dark background for nighttime reading",
      },
      system: {
        label: "Follow system",
        description: "Automatically follow the system theme",
      },
    },
    randomPicks: {
      title: "Show random picks",
      description: "Show a random picks carousel at the top of the library",
    },
    rowsPerPage: {
      title: "Rows per page",
      description:
        "Rows shown per library page; columns adapt to the screen width",
    },
  },
} as const;
