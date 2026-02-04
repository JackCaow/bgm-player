# Shadcn-vue Select 组件使用说明

## 概述

已成功将预设配置选择器从原生 `<select>` 元素升级为 shadcn-vue 的 Select 组件，提供更好的用户体验和可访问性。

## 组件结构

创建的 Select 组件包含以下子组件：

```
src/components/ui/select/
├── index.ts              # 导出所有组件和样式变体
├── Select.vue            # 根组件（基于 reka-ui SelectRoot）
├── SelectTrigger.vue     # 触发器按钮
├── SelectValue.vue       # 显示当前选中值
├── SelectContent.vue     # 下拉内容容器
├── SelectItem.vue        # 选项项
├── SelectGroup.vue       # 选项分组
└── SelectLabel.vue       # 分组标签
```

## 基本用法

### 简单示例

```vue
<script setup lang="ts">
import { ref } from "vue";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const selectedValue = ref("");
</script>

<template>
  <Select v-model="selectedValue">
    <SelectTrigger>
      <SelectValue placeholder="请选择..." />
    </SelectTrigger>
    <SelectContent>
      <SelectItem value="option1">选项 1</SelectItem>
      <SelectItem value="option2">选项 2</SelectItem>
      <SelectItem value="option3">选项 3</SelectItem>
    </SelectContent>
  </Select>
</template>
```

### 带分组的示例

```vue
<template>
  <Select v-model="selectedValue">
    <SelectTrigger>
      <SelectValue placeholder="选择预设" />
    </SelectTrigger>
    <SelectContent>
      <SelectGroup>
        <SelectLabel>内置预设</SelectLabel>
        <SelectItem value="preset1">高质量</SelectItem>
        <SelectItem value="preset2">快速处理</SelectItem>
      </SelectGroup>
      <SelectGroup>
        <SelectLabel>自定义预设</SelectLabel>
        <SelectItem value="custom1">我的配置 1</SelectItem>
        <SelectItem value="custom2">我的配置 2</SelectItem>
      </SelectGroup>
    </SelectContent>
  </Select>
</template>
```

## 在 SettingsView 中的实际应用

```vue
<Select
  :model-value="currentPreset?.id || ''"
  @update:model-value="handleApplyPreset"
>
  <SelectTrigger class="preset-select-trigger">
    <SelectValue placeholder="选择预设" />
  </SelectTrigger>
  <SelectContent>
    <SelectGroup>
      <SelectLabel>内置预设</SelectLabel>
      <SelectItem
        v-for="preset in presets.filter(p => p.isBuiltIn)"
        :key="preset.id"
        :value="preset.id"
      >
        {{ preset.name }}{{ preset.isDefault ? ' (默认)' : '' }}
      </SelectItem>
    </SelectGroup>
    <SelectGroup v-if="presets.some(p => !p.isBuiltIn)">
      <SelectLabel>自定义预设</SelectLabel>
      <SelectItem
        v-for="preset in presets.filter(p => !p.isBuiltIn)"
        :key="preset.id"
        :value="preset.id"
      >
        {{ preset.name }}
      </SelectItem>
    </SelectGroup>
  </SelectContent>
</Select>
```

## 特性

### ✅ 已实现的功能

1. **可访问性**
   - 完整的键盘导航支持（方向键、Enter、Escape）
   - ARIA 属性支持
   - 屏幕阅读器友好

2. **视觉效果**
   - 平滑的打开/关闭动画
   - 选中状态指示器（✓ 图标）
   - Hover 和 Focus 状态
   - 响应式设计

3. **功能特性**
   - 支持分组（SelectGroup + SelectLabel）
   - 自定义占位符
   - 禁用选项支持
   - 双向数据绑定（v-model）

4. **样式定制**
   - 基于 Tailwind CSS
   - 支持深色/浅色主题
   - 可通过 class 属性自定义样式
   - 使用 CVA (class-variance-authority) 管理变体

## API 参考

### Select Props

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| modelValue | string | - | 当前选中的值 |
| disabled | boolean | false | 是否禁用 |
| name | string | - | 表单字段名称 |

### Select Events

| 事件 | 参数 | 说明 |
|------|------|------|
| update:modelValue | (value: string) | 选中值变化时触发 |

### SelectTrigger Props

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| variant | "default" | "default" | 样式变体 |
| class | string | - | 自定义类名 |

### SelectContent Props

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| position | "popper" \| "item-aligned" | "popper" | 定位模式 |
| sideOffset | number | 4 | 与触发器的距离 |
| class | string | - | 自定义类名 |

### SelectItem Props

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| value | string | - | 选项值（必填） |
| disabled | boolean | false | 是否禁用 |
| class | string | - | 自定义类名 |

## 样式定制

### 修改触发器样式

```vue
<SelectTrigger class="w-full h-12 text-lg">
  <SelectValue />
</SelectTrigger>
```

### 修改内容容器样式

```vue
<SelectContent class="max-h-[300px]">
  <!-- items -->
</SelectContent>
```

### 修改选项样式

```vue
<SelectItem value="option1" class="text-red-500 font-bold">
  特殊选项
</SelectItem>
```

## 主题支持

Select 组件完全支持深色/浅色主题，使用 Tailwind CSS 的 CSS 变量系统：

```css
/* 浅色主题 */
:root {
  --background: 0 0% 100%;
  --foreground: 0 0% 3.9%;
  --popover: 0 0% 100%;
  --popover-foreground: 0 0% 3.9%;
  /* ... */
}

/* 深色主题 */
.dark {
  --background: 0 0% 3.9%;
  --foreground: 0 0% 98%;
  --popover: 0 0% 3.9%;
  --popover-foreground: 0 0% 98%;
  /* ... */
}
```

## 依赖

- `reka-ui`: ^2.8.0 - 基础组件库
- `lucide-vue-next`: ^0.563.0 - 图标库（ChevronDown, Check）
- `class-variance-authority`: ^0.7.1 - 样式变体管理
- `clsx`: ^2.1.1 - 类名合并
- `tailwind-merge`: ^3.4.0 - Tailwind 类名合并

## 与原生 select 的对比

| 特性 | 原生 select | shadcn Select |
|------|-------------|---------------|
| 样式定制 | ❌ 受限 | ✅ 完全可定制 |
| 动画效果 | ❌ 无 | ✅ 平滑动画 |
| 键盘导航 | ✅ 基础 | ✅ 增强 |
| 可访问性 | ✅ 基础 | ✅ 完整 ARIA |
| 分组支持 | ✅ optgroup | ✅ SelectGroup |
| 图标支持 | ❌ 无 | ✅ 内置图标 |
| 主题支持 | ❌ 受限 | ✅ 完整支持 |
| 移动端体验 | ⚠️ 系统默认 | ✅ 统一体验 |

## 最佳实践

1. **始终提供 placeholder**
   ```vue
   <SelectValue placeholder="请选择..." />
   ```

2. **使用分组提高可读性**
   ```vue
   <SelectGroup>
     <SelectLabel>分组标题</SelectLabel>
     <!-- items -->
   </SelectGroup>
   ```

3. **为长列表设置最大高度**
   ```vue
   <SelectContent class="max-h-[300px]">
     <!-- items -->
   </SelectContent>
   ```

4. **提供有意义的选项文本**
   ```vue
   <SelectItem value="id">显示名称</SelectItem>
   ```

5. **处理空状态**
   ```vue
   <SelectContent>
     <SelectItem v-if="items.length === 0" value="" disabled>
       暂无选项
     </SelectItem>
     <SelectItem v-for="item in items" :key="item.id" :value="item.id">
       {{ item.name }}
     </SelectItem>
   </SelectContent>
   ```

## 故障排除

### 下拉菜单不显示

确保已正确导入 SelectContent 和 SelectPortal：

```vue
import { SelectContent } from "@/components/ui/select";
```

### 样式不正确

确保 Tailwind CSS 已正确配置，并且 CSS 变量已定义在 `src/style.css` 中。

### 选中状态不更新

使用 `v-model` 或 `:model-value` + `@update:model-value`：

```vue
<!-- 推荐：v-model -->
<Select v-model="value">

<!-- 或者：显式绑定 -->
<Select :model-value="value" @update:model-value="handleChange">
```

## 更新日志

### 2024-02-04
- ✅ 创建完整的 shadcn-vue Select 组件
- ✅ 集成到 SettingsView 的预设选择器
- ✅ 支持分组和标签
- ✅ 完整的主题支持
- ✅ 可访问性优化
