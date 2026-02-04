import { createI18n } from 'vue-i18n';

const messages = {
  en: {
    app: {
      name: 'BGM Extractor',
    },
    nav: {
      queue: 'Queue',
      result: 'Results',
      history: 'History',
      merge: 'Merge',
      settings: 'Settings',
    },
    sidebar: {
      fileList: 'Files',
      emptyFiles: 'Drop or click to add files',
      startProcess: 'Start Processing',
      processing: 'Processing',
    },
    queue: {
      title: 'Processing Queue',
      pending: '{count} pending',
      dropTitle: 'Drop audio files here',
      dropSubtitle: 'or click to select files',
      dropFormats: 'Supports MP3, WAV, FLAC, M4A, OGG, AAC',
      waitingProcess: 'Waiting to process',
    },
    result: {
      title: 'Results',
      clearCompleted: 'Clear Completed',
      emptyHint: 'Select a completed file to view results',
      bgm: 'Background Music (BGM)',
      vocals: 'Vocals Track',
    },
    merge: {
      title: 'Merge Tracks',
      reset: 'Reset',
      mergeButton: 'Merge & Export',
      merging: 'Merging...',
      success: 'Merge Complete',
      selectFiles: 'Select Audio Files',
      track1: 'Track 1 (BGM)',
      track2: 'Track 2 (Vocals)',
      clickToSelect: 'Click to select file',
      volumeControl: 'Volume Control',
      clearFiles: 'Clear All',
      trackList: 'Track List',
      tracksCount: 'tracks',
      addTrack: 'Add Audio Track',
      addTrackHint: 'Click to select audio files',
      emptyTracks: 'Add at least 2 tracks to merge',
    },
    history: {
      title: 'History',
      clearAll: 'Clear All',
      emptyHint: 'No processing history yet',
      processedAt: 'Processed At',
      model: 'Model',
      processingTime: 'Processing Time',
      sourceFile: 'Source File',
      outputFiles: 'Output Files',
      openFolder: 'Open Folder',
      delete: 'Delete',
    },
    settings: {
      title: 'Settings',
      model: 'Separation Model',
      outputDir: 'Output Directory',
      optional: 'Optional',
      defaultOutput: 'Default: Documents/BGM Extractor Output',
      theme: 'Theme',
      themeLight: 'Light',
      themeDark: 'Dark',
      themeSystem: 'System',
      language: 'Language',
      exportFormat: 'Export Format',
      audioQuality: 'Audio Quality',
      sampleRate: 'Sample Rate',
      bitrate: 'Bitrate',
      separationMode: 'Separation Mode',
      autoSelectMode: 'Auto-select based on model',
    },
    separation: {
      mode: 'Separation Mode',
      twoTrack: '2 Tracks (Vocals + BGM)',
      fourTrack: '4 Tracks (Vocals/Drums/Bass/Other)',
      sixTrack: '6 Tracks (Vocals/Drums/Bass/Guitar/Piano/Other)',
      autoSelect: 'Auto-select based on model',
    },
    tracks: {
      vocals: 'Vocals',
      drums: 'Drums',
      bass: 'Bass',
      other: 'Other',
      guitar: 'Guitar',
      piano: 'Piano',
      no_vocals: 'BGM',
      all: 'All Tracks',
    },
    format: {
      wav: 'WAV (Lossless)',
      mp3: 'MP3 (Compressed)',
      flac: 'FLAC (Lossless)',
      aac: 'AAC (Compressed)',
      m4a: 'M4A (Apple)',
      ogg: 'OGG (Vorbis)',
      opus: 'OPUS (Modern)',
    },
    quality: {
      low: 'Low (128 kbps)',
      medium: 'Medium (192 kbps)',
      high: 'High (256 kbps)',
      lossless: 'Lossless (320 kbps)',
    },
    player: {
      play: 'Play',
      pause: 'Pause',
      stop: 'Stop',
      playBgm: 'Play BGM',
      playVocals: 'Play Vocals',
      playBoth: 'Play Both',
      volume: 'Volume',
      bgmVolume: 'BGM Volume',
      vocalsVolume: 'Vocals Volume',
      showPlayer: 'Show Player',
      close: 'Close',
    },
    model: {
      htdemucs: {
        desc: 'Recommended · Balanced speed and quality',
      },
      htdemucs_ft: {
        desc: 'Professional · Highest quality',
      },
      htdemucs_6s: {
        desc: '6-Track · Vocals/Drums/Bass/Guitar/Piano/Other',
      },
      mdx_extra: {
        desc: 'Alternative · MDX Enhanced',
      },
      speed: {
        fast: 'Fast',
        medium: 'Medium',
        slow: 'Slow',
      },
      quality: {
        excellent: 'Excellent',
        top: 'Top',
      },
    },
    status: {
      pending: 'Pending',
      processing: 'Processing',
      done: 'Done',
      error: 'Error',
    },
    drag: {
      release: 'Release to add files',
    },
    confirm: {
      cancel: 'Cancel',
      confirm: 'Confirm',
      clearCompleted: {
        title: 'Clear Completed Files',
        message: 'Are you sure you want to clear all completed files? This action cannot be undone.',
      },
      clearHistory: {
        title: 'Clear All History',
        message: 'Are you sure you want to clear all history records? This action cannot be undone.',
      },
      deleteHistory: {
        title: 'Delete History Item',
        message: 'Are you sure you want to delete this history item? This action cannot be undone.',
      },
      cancelAll: {
        title: 'Cancel All Tasks',
        message: 'Are you sure you want to cancel all pending and processing tasks?',
      },
      clearTracks: {
        title: 'Clear All Tracks',
        message: 'Are you sure you want to clear all tracks from the merge list?',
      },
      removeFile: {
        title: 'Remove File',
        message: 'Are you sure you want to remove this file from the queue?',
      },
    },
    project: {
      title: 'Projects',
      new: 'New Project',
      save: 'Save Project',
      load: 'Load Project',
      export: 'Export Project',
      import: 'Import Project',
      rename: 'Rename',
      delete: 'Delete',
      switch: 'Switch Project',
      current: 'Current Project',
      name: 'Project Name',
      created: 'Created',
      modified: 'Last Modified',
      files: 'Files',
      stats: 'Statistics',
      totalProcessed: 'Total Processed',
      totalFailed: 'Total Failed',
      totalTime: 'Total Time',
      newDialog: {
        title: 'New Project',
        namePlaceholder: 'e.g., My Music Project',
      },
      renameDialog: {
        title: 'Rename Project',
        namePlaceholder: 'Enter new name',
      },
      deleteConfirm: {
        title: 'Delete Project',
        message: 'Are you sure you want to delete this project? All files and settings will be lost.',
      },
      switchConfirm: {
        title: 'Switch Project',
        message: 'Current progress will be saved. Continue?',
      },
    },
  },
  zh: {
    app: {
      name: 'BGM 提取器',
    },
    nav: {
      queue: '队列',
      result: '结果',
      history: '历史',
      merge: '合并',
      settings: '设置',
    },
    sidebar: {
      fileList: '文件列表',
      emptyFiles: '拖放或点击添加文件',
      startProcess: '开始处理',
      processing: '处理中',
    },
    queue: {
      title: '处理队列',
      pending: '{count} 个待处理',
      dropTitle: '拖放音频文件到这里',
      dropSubtitle: '或点击选择文件',
      dropFormats: '支持 MP3, WAV, FLAC, M4A, OGG, AAC',
      waitingProcess: '等待处理',
    },
    result: {
      title: '处理结果',
      clearCompleted: '清除已完成',
      emptyHint: '选择已完成的文件查看结果',
      bgm: '背景音乐 (BGM)',
      vocals: '人声轨道',
    },
    merge: {
      title: '音轨合并',
      reset: '重置',
      mergeButton: '合并导出',
      merging: '合并中...',
      success: '合并完成',
      selectFiles: '选择音频文件',
      track1: '音轨 1 (BGM)',
      track2: '音轨 2 (人声)',
      clickToSelect: '点击选择文件',
      volumeControl: '音量控制',
      clearFiles: '清空全部',
      trackList: '音轨列表',
      tracksCount: '个音轨',
      addTrack: '添加音轨',
      addTrackHint: '点击选择音频文件',
      emptyTracks: '至少添加 2 个音轨才能合并',
    },
    history: {
      title: '历史记录',
      clearAll: '清空全部',
      emptyHint: '暂无处理历史',
      processedAt: '处理时间',
      model: '模型',
      processingTime: '处理耗时',
      sourceFile: '源文件',
      outputFiles: '输出文件',
      openFolder: '打开文件夹',
      delete: '删除',
    },
    settings: {
      title: '设置',
      model: '分离模型',
      outputDir: '输出目录',
      optional: '可选',
      defaultOutput: '默认: 文档/BGM Extractor Output',
      theme: '主题',
      themeLight: '浅色',
      themeDark: '深色',
      themeSystem: '跟随系统',
      language: '语言',
      exportFormat: '导出格式',
      audioQuality: '音频质量',
      sampleRate: '采样率',
      bitrate: '比特率',
      separationMode: '分离模式',
      autoSelectMode: '根据模型自动选择',
    },
    separation: {
      mode: '分离模式',
      twoTrack: '2 轨 (人声 + BGM)',
      fourTrack: '4 轨 (人声/鼓/贝斯/其他)',
      sixTrack: '6 轨 (人声/鼓/贝斯/吉他/钢琴/其他)',
      autoSelect: '根据模型自动选择',
    },
    tracks: {
      vocals: '人声',
      drums: '鼓',
      bass: '贝斯',
      other: '其他',
      guitar: '吉他',
      piano: '钢琴',
      no_vocals: 'BGM',
      all: '全部音轨',
    },
    format: {
      wav: 'WAV (无损)',
      mp3: 'MP3 (压缩)',
      flac: 'FLAC (无损)',
      aac: 'AAC (压缩)',
      m4a: 'M4A (苹果)',
      ogg: 'OGG (Vorbis)',
      opus: 'OPUS (现代)',
    },
    quality: {
      low: '低 (128 kbps)',
      medium: '中 (192 kbps)',
      high: '高 (256 kbps)',
      lossless: '无损 (320 kbps)',
    },
    player: {
      play: '播放',
      pause: '暂停',
      stop: '停止',
      playBgm: '播放背景音乐',
      playVocals: '播放人声',
      playBoth: '播放混合',
      volume: '音量',
      bgmVolume: 'BGM 音量',
      vocalsVolume: '人声音量',
      showPlayer: '显示播放器',
      close: '关闭',
    },
    model: {
      htdemucs: {
        desc: '推荐 · 速度与质量平衡',
      },
      htdemucs_ft: {
        desc: '专业级 · 最高质量',
      },
      htdemucs_6s: {
        desc: '6 轨分离 · 人声/鼓/贝斯/吉他/钢琴/其他',
      },
      mdx_extra: {
        desc: '备选 · MDX 增强版',
      },
      speed: {
        fast: '快',
        medium: '中',
        slow: '慢',
      },
      quality: {
        excellent: '优秀',
        top: '顶级',
      },
    },
    status: {
      pending: '等待中',
      processing: '处理中',
      done: '已完成',
      error: '错误',
    },
    drag: {
      release: '释放以添加文件',
    },
    confirm: {
      cancel: '取消',
      confirm: '确认',
      clearCompleted: {
        title: '清除已完成文件',
        message: '确定要清除所有已完成的文件吗？此操作无法撤销。',
      },
      clearHistory: {
        title: '清空历史记录',
        message: '确定要清空所有历史记录吗？此操作无法撤销。',
      },
      deleteHistory: {
        title: '删除历史记录',
        message: '确定要删除这条历史记录吗？此操作无法撤销。',
      },
      cancelAll: {
        title: '取消所有任务',
        message: '确定要取消所有待处理和正在处理的任务吗？',
      },
      clearTracks: {
        title: '清空音轨列表',
        message: '确定要清空合并列表中的所有音轨吗？',
      },
      removeFile: {
        title: '移除文件',
        message: '确定要从队列中移除这个文件吗？',
      },
    },
    project: {
      title: '项目',
      new: '新建项目',
      save: '保存项目',
      load: '加载项目',
      export: '导出项目',
      import: '导入项目',
      rename: '重命名',
      delete: '删除',
      switch: '切换项目',
      current: '当前项目',
      name: '项目名称',
      created: '创建时间',
      modified: '最后修改',
      files: '文件',
      stats: '统计',
      totalProcessed: '已处理',
      totalFailed: '失败',
      totalTime: '总耗时',
      newDialog: {
        title: '新建项目',
        namePlaceholder: '例如：我的音乐项目',
      },
      renameDialog: {
        title: '重命名项目',
        namePlaceholder: '输入新名称',
      },
      deleteConfirm: {
        title: '删除项目',
        message: '确定要删除这个项目吗？所有文件和设置都将丢失。',
      },
      switchConfirm: {
        title: '切换项目',
        message: '当前进度将被保存。是否继续？',
      },
    },
  },
};

// Get saved locale or detect from system
function getDefaultLocale(): string {
  const saved = localStorage.getItem('locale');
  if (saved) return saved;

  const browserLang = navigator.language.toLowerCase();
  if (browserLang.startsWith('zh')) return 'zh';
  return 'en';
}

export const i18n = createI18n({
  legacy: false,
  locale: getDefaultLocale(),
  fallbackLocale: 'en',
  messages,
});

export function setLocale(locale: string) {
  i18n.global.locale.value = locale as 'zh' | 'en';
  localStorage.setItem('locale', locale);
}

export function getLocale(): string {
  return i18n.global.locale.value;
}
