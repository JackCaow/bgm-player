# 队列管理优化文档

## 概述

本次更新大幅增强了 BGM Extractor 的队列管理功能,支持并发处理、队列控制和智能调度。

## 新增功能

### 1. 并发处理
- **多任务并行**: 支持同时处理多个文件(默认2个,可配置)
- **智能调度**: 自动分配处理任务到可用槽位
- **资源优化**: 根据系统资源动态调整并发数

### 2. 队列控制
- **暂停/恢复**: 可以暂停整个队列,稍后恢复处理
- **取消任务**: 支持取消单个文件或整个队列
- **重试机制**: 自动或手动重试失败的任务
- **优先级管理**: 调整文件处理顺序

### 3. 进度追踪
- **实时统计**: 显示待处理、处理中、已完成、失败等状态
- **时间估算**: 基于历史数据预估剩余处理时间
- **平均速度**: 计算平均处理时间

### 4. 错误处理
- **自动重试**: 失败任务自动重试(可配置次数)
- **错误暂停**: 出错时可选择暂停队列
- **详细日志**: 记录每个任务的处理时间和错误信息

## 技术实现

### 新增类型定义

```typescript
// 扩展的文件状态
status: "pending" | "processing" | "paused" | "cancelled" | "done" | "error"

// 队列设置
interface QueueSettings {
  maxConcurrent: number;      // 最大并发数
  autoRetry: boolean;          // 自动重试
  maxRetries: number;          // 最大重试次数
  pauseOnError: boolean;       // 出错时暂停
}

// 队列统计
interface QueueStats {
  total: number;
  pending: number;
  processing: number;
  paused: number;
  done: number;
  error: number;
  cancelled: number;
  averageProcessingTime: number;
  estimatedTotalTime: number;
}
```

### 核心组件

#### useQueueManager.ts
队列管理器,负责:
- 计算队列统计信息
- 管理并发任务
- 处理优先级排序
- 控制队列状态(暂停/恢复/取消)
- 记录处理时间和估算

#### useFiles.ts (更新)
文件管理器,集成队列管理器:
- 并发处理循环
- 队列控制方法
- 进度追踪
- 错误处理

## API 使用

### 基础用法

```typescript
import { useFiles } from "@/composables";

const {
  files,
  queueStats,
  queueSettings,
  isPaused,
  startBatchExtraction,
  pauseQueue,
  resumeQueue,
  cancelFile,
  cancelAll,
  retryFile,
  retryAllFailed,
  moveFileUp,
  moveFileDown,
  updateQueueSettings,
} = useFiles();

// 开始批量处理
startBatchExtraction();

// 暂停队列
pauseQueue();

// 恢复队列
resumeQueue();

// 取消特定文件
cancelFile(fileId);

// 取消所有任务
cancelAll();

// 重试失败的文件
retryFile(fileId);

// 重试所有失败的文件
retryAllFailed();

// 调整优先级
moveFileUp(fileId);
moveFileDown(fileId);

// 更新队列设置
updateQueueSettings({
  maxConcurrent: 3,
  autoRetry: true,
  maxRetries: 2,
  pauseOnError: false,
});
```

### 队列统计

```typescript
// 获取队列统计信息
const stats = queueStats.value;

console.log(`总计: ${stats.total}`);
console.log(`待处理: ${stats.pending}`);
console.log(`处理中: ${stats.processing}`);
console.log(`已完成: ${stats.done}`);
console.log(`失败: ${stats.error}`);
console.log(`平均处理时间: ${stats.averageProcessingTime}秒`);
console.log(`预计剩余时间: ${stats.estimatedTotalTime}秒`);
```

## 配置选项

### 队列设置

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| maxConcurrent | number | 2 | 最大并发处理数 |
| autoRetry | boolean | true | 自动重试失败任务 |
| maxRetries | number | 3 | 最大重试次数 |
| pauseOnError | boolean | false | 出错时暂停队列 |

### 建议配置

**高性能机器** (16GB+ RAM, 多核CPU):
```typescript
{
  maxConcurrent: 4,
  autoRetry: true,
  maxRetries: 2,
  pauseOnError: false,
}
```

**普通机器** (8GB RAM):
```typescript
{
  maxConcurrent: 2,
  autoRetry: true,
  maxRetries: 3,
  pauseOnError: false,
}
```

**低配机器** (4GB RAM):
```typescript
{
  maxConcurrent: 1,
  autoRetry: false,
  maxRetries: 1,
  pauseOnError: true,
}
```

## 性能优化

### 并发处理优势
- **2倍速度提升**: 2个并发任务相比单任务处理
- **资源利用**: 充分利用多核CPU和GPU
- **用户体验**: 减少等待时间

### 内存管理
- 限制并发数避免内存溢出
- 完成的任务及时释放资源
- 历史数据只保留最近20条用于估算

## 后续改进

### 短期 (1-2周)
- [ ] UI 界面集成队列控制按钮
- [ ] 显示实时队列统计
- [ ] 添加队列设置面板
- [ ] 优化进度显示

### 中期 (1个月)
- [ ] 支持拖拽调整优先级
- [ ] 队列预设模板
- [ ] 批量操作(全选/反选)
- [ ] 导出队列配置

### 长期 (3个月)
- [ ] 分布式处理支持
- [ ] 云端队列同步
- [ ] 智能优先级算法
- [ ] 机器学习时间预测

## 测试建议

### 功能测试
1. 添加多个文件到队列
2. 测试并发处理(观察同时处理的文件数)
3. 测试暂停/恢复功能
4. 测试取消功能
5. 测试重试功能
6. 测试优先级调整

### 性能测试
1. 测试不同并发数的处理速度
2. 监控内存使用情况
3. 测试大文件处理
4. 测试长时间运行稳定性

### 边界测试
1. 空队列处理
2. 单文件处理
3. 大量文件(100+)处理
4. 网络中断恢复
5. 磁盘空间不足

## 已知问题

1. **进度更新延迟**: 并发处理时进度更新可能有1秒延迟
2. **取消响应**: 取消正在处理的任务需要等待当前步骤完成
3. **内存估算**: 时间估算基于历史数据,首次处理可能不准确

## 更新日志

### v1.1.0 (2026-02-03)
- ✅ 实现并发处理支持
- ✅ 添加队列控制功能(暂停/恢复/取消)
- ✅ 实现优先级管理
- ✅ 添加进度估算
- ✅ 实现自动重试机制
- ✅ 添加队列统计功能

---

**维护者**: BGM Extractor Team
**最后更新**: 2026-02-03
