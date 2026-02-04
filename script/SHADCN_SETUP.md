# shadcn-vue Setup

This project has been configured with shadcn-vue, a Vue 3 component library built with Radix Vue and Tailwind CSS.

## What's Installed

- **Tailwind CSS**: Utility-first CSS framework
- **Radix Vue**: Unstyled, accessible component primitives
- **class-variance-authority**: For creating variant-based component APIs
- **clsx & tailwind-merge**: For conditional className handling

## Project Structure

```
src/
├── components/
│   └── ui/              # shadcn-vue components
│       ├── Button.vue   # Example button component
│       └── index.ts     # Component exports
├── lib/
│   └── utils.ts         # Utility functions (cn helper)
└── style.css            # Global styles with Tailwind directives
```

## Configuration Files

- `tailwind.config.js` - Tailwind CSS configuration with shadcn-vue theme
- `postcss.config.js` - PostCSS configuration for Tailwind
- `components.json` - shadcn-vue CLI configuration

## Adding Components

To add more shadcn-vue components manually:

1. Visit https://www.shadcn-vue.com/docs/components
2. Copy the component code
3. Create a new file in `src/components/ui/`
4. Export it from `src/components/ui/index.ts`

## Usage Example

```vue
<script setup lang="ts">
import { Button } from "@/components/ui";
</script>

<template>
  <Button variant="default">Click me</Button>
  <Button variant="outline">Outline</Button>
  <Button variant="ghost" size="sm">Small Ghost</Button>
</template>
```

## Available Button Variants

- `default` - Primary green button
- `destructive` - Red destructive action
- `outline` - Outlined button
- `secondary` - Secondary style
- `ghost` - Transparent with hover
- `link` - Link style

## Available Button Sizes

- `default` - Standard size
- `sm` - Small
- `lg` - Large
- `icon` - Square icon button

## Theme Integration

The shadcn-vue theme has been configured to work alongside your existing Spotify-inspired theme. The components use CSS variables that can be customized in `src/style.css`.

## Next Steps

You can now:
1. Use the Button component in your views
2. Add more components from shadcn-vue (Card, Dialog, Select, etc.)
3. Customize the theme colors in `tailwind.config.js`
4. Create custom variants for your components
