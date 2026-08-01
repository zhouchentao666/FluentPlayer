<template>
  <WinComboBox
    :model-value="modelValue"
    :items="items"
    :item-text="itemText"
    :item-value="itemValue"
    :placeholder="placeholder"
    :disabled="disabled"
    :filterable="filterable"
    :filter-placeholder="filterPlaceholder"
    :error="error"
    @update:model-value="$emit('update:modelValue', $event)"
    @change="$emit('change', $event)"
  >
    <template #display="{ item }">
      <slot name="display" :item="item" />
    </template>
    <template #default="{ item, index }">
      <slot :item="item" :index="index" />
    </template>
  </WinComboBox>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import WinComboBox from '../winui/WinComboBox.vue';

export default defineComponent({
  name: 'SelectBox',
  components: { WinComboBox },
  props: {
    modelValue: { type: [String, Number, Object, null] as any, default: null },
    items: { type: Array as PropType<any[]>, required: true },
    itemText: { type: Function as PropType<(item: any) => string>, default: (item: any) => item },
    itemValue: { type: Function as PropType<(item: any) => any>, default: (item: any) => item },
    placeholder: { type: String, default: 'Select an option' },
    disabled: { type: Boolean, default: false },
    filterable: { type: Boolean, default: false },
    filterPlaceholder: { type: String, default: 'Filter' },
    error: { type: String, default: '' },
  },
  emits: ['update:modelValue', 'change'],
});
</script>
