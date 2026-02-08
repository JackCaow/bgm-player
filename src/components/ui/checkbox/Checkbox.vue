<script setup lang="ts">
import type { CheckboxRootEmits, CheckboxRootProps } from "reka-ui"
import type { HTMLAttributes } from "vue"
import { reactiveOmit } from "@vueuse/core"
import { Check } from "lucide-vue-next"
import { CheckboxIndicator, CheckboxRoot, useForwardPropsEmits } from "reka-ui"
import { cn } from "@/lib/utils"

const props = defineProps<CheckboxRootProps & { class?: HTMLAttributes["class"] }>()
const emits = defineEmits<CheckboxRootEmits>()

const delegatedProps = reactiveOmit(props, "class")

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <CheckboxRoot
    v-bind="forwarded"
    :class="
      cn('grid place-content-center peer h-[18px] w-[18px] shrink-0 rounded-[6px] border border-[color:var(--border)] bg-[color:var(--bg)] text-[color:var(--text)] shadow-sm shadow-black/10 transition-colors hover:border-[color:var(--primary)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[color:var(--bg)] disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-[color:var(--primary)] data-[state=checked]:border-[color:var(--primary)] data-[state=checked]:text-black data-[state=indeterminate]:bg-[color:var(--primary)] data-[state=indeterminate]:border-[color:var(--primary)] data-[state=indeterminate]:text-black',
         props.class)"
  >
    <CheckboxIndicator class="grid place-content-center text-current">
      <slot>
        <Check class="h-[14px] w-[14px]" />
      </slot>
    </CheckboxIndicator>
  </CheckboxRoot>
</template>
