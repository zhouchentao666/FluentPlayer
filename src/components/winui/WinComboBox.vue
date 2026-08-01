<template>
  <div
    class="win-combo-box"
    :class="{ 'is-open': isOpen, 'is-disabled': disabled }"
    ref="root"
  >
    <div
      class="combo-box-header"
      :class="{ 'is-error': !!error }"
      @click="toggleOpen"
      role="combobox"
      :aria-expanded="isOpen"
      :aria-disabled="disabled"
    >
      <WinTextBlock class="combo-box-display">
        <span v-if="selectedItem && !hideDisplay">{{ displayText }}</span>
        <span v-else-if="placeholder && !selectedItem" class="combo-box-placeholder">{{ placeholder }}</span>
        <slot v-else name="display" :item="selectedItem" />
      </WinTextBlock>

      <svg class="combo-box-glyph" :class="{ 'is-flipped': isOpen }" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
        <path d="M8 11L3 6h10z" fill="currentColor" />
      </svg>
    </div>

    <transition name="combo-box-popup">
      <div v-if="isOpen" class="combo-box-popup" v-click-outside="closePopup">
        <div v-if="filterable" class="combo-box-filter">
          <WinTextBox v-model="filterText" :placeholder="filterPlaceholder" />
        </div>
        <WinScrollViewer class="combo-box-list" :class="{ 'is-scroll': !hideScroll }">
          <ul role="listbox" :aria-activedescendant="activeDescendant">
            <li
              v-for="(item, index) in filteredItems"
              :key="getItemValue(item)"
              class="combo-box-item"
              :class="{ 'is-selected': isSelected(item), 'is-active': activeIndex === index }"
              :id="`${uid}-${index}`"
              role="option"
              :aria-selected="isSelected(item)"
              @click="selectItem(item)"
              @mouseenter="activeIndex = index"
            >
              <slot :item="item" :index="index">
                <WinTextBlock>{{ getItemText(item) }}</WinTextBlock>
              </slot>
            </li>
          </ul>
        </WinScrollViewer>
      </div>
    </transition>

    <div v-if="error" class="combo-box-error" role="alert">{{ error }}</div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, computed, PropType, onMounted, nextTick, watch } from 'vue';
import WinTextBlock from './WinTextBlock.vue';
import WinTextBox from './WinTextBox.vue';
import WinScrollViewer from './WinScrollViewer.vue';

interface ClickOutsideDirective {
  mounted(el: HTMLElement, binding: any): void;
}

const vClickOutside: any = {
  mounted(el: HTMLElement, binding: any) {
    const handler = (e: MouseEvent) => {
      if (el && !el.contains(e.target as Node) && !(binding.value as Function)()) {
        binding.value();
      }
    };
    (el as any)._clickOutsideHandler = handler;
    document.addEventListener('click', handler, true);
  },
  unmounted(el: HTMLElement) {
    document.removeEventListener('click', (el as any)._clickOutsideHandler, true);
  },
};

export default defineComponent({
  name: 'WinComboBox',
  components: { WinTextBlock, WinTextBox, WinScrollViewer },
  directives: { clickOutside: vClickOutside },
  props: {
    modelValue: { type: [String, Number, Object, null] as any, default: null },
    items: { type: Array as PropType<any[]>, required: true },
    itemText: { type: Function as PropType<(item: any) => string>, default: (item: any) => item },
    itemValue: { type: Function as PropType<(item: any) => any>, default: (item: any) => item },
    placeholder: { type: String, default: 'Select an option' },
    disabled: { type: Boolean, default: false },
    filterable: { type: Boolean, default: false },
    filterPlaceholder: { type: String, default: 'Filter' },
    hideDisplay: { type: Boolean, default: false },
    hideScroll: { type: Boolean, default: false },
    error: { type: String, default: '' },
  },
  emits: ['update:modelValue', 'change'],
  setup(props, { emit }) {
    const root = ref<HTMLElement | null>(null);
    const isOpen = ref(false);
    const filterText = ref('');
    const activeIndex = ref(-1);
    const uid = `cbox-${Math.random().toString(36).slice(2, 9)}`;

    const getItemText = (item: any) => {
      if (item == null) return '';
      if (typeof props.itemText === 'function') return props.itemText(item);
      if (typeof item === 'object' && item.text != null) return item.text;
      if (typeof item === 'string' || typeof item === 'number') return String(item);
      return '';
    };
    const getItemValue = (item: any) => {
      if (item == null) return null;
      if (typeof props.itemValue === 'function') return props.itemValue(item);
      if (typeof item === 'object' && item.value != null) return item.value;
      return item;
    };
    const selectedItem = computed(() => {
      if (props.modelValue == null) return null;
      return props.items.find((it) => getItemValue(it) === props.modelValue) ?? null;
    });
    const displayText = computed(() =>
      selectedItem.value != null ? getItemText(selectedItem.value) : ''
    );
    const filteredItems = computed(() => {
      if (!props.filterable || !filterText.value) return props.items;
      const f = filterText.value.toLowerCase();
      return props.items.filter((it) => getItemText(it).toLowerCase().includes(f));
    });
    const isSelected = (item: any) =>
      selectedItem.value != null && getItemValue(item) === getItemValue(selectedItem.value);
    const activeDescendant = computed(() =>
      activeIndex.value >= 0 ? `${uid}-${activeIndex.value}` : ''
    );

    const toggleOpen = () => {
      if (props.disabled) return;
      isOpen.value = !isOpen.value;
      if (isOpen.value) {
        activeIndex.value = -1;
        nextTick(() => {
          const input = root.value?.querySelector('.combo-box-filter input');
          (input as HTMLInputElement | null)?.focus();
        });
      }
    };
    const closePopup = () => {
      isOpen.value = false;
    };
    const selectItem = (item: any) => {
      emit('update:modelValue', getItemValue(item));
      emit('change', getItemValue(item));
      isOpen.value = false;
    };

    onMounted(() => {
      document.addEventListener;
    });

    return {
      root,
      isOpen,
      filterText,
      activeIndex,
      uid,
      getItemText,
      getItemValue,
      selectedItem,
      displayText,
      filteredItems,
      isSelected,
      activeDescendant,
      toggleOpen,
      closePopup,
      selectItem,
    };
  },
});
</script>

<style scoped>
.win-combo-box {
  position: relative;
  width: 100%;
  font-family: var(--fluent-font-family, 'Segoe UI', sans-serif);
}
.combo-box-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 12px;
  min-height: 36px;
  background: var(--card-bg, #fff);
  border: 1px solid var(--fluent-border, #d1d1d1);
  border-radius: 4px;
  cursor: pointer;
  user-select: none;
  transition: border-color 0.15s ease, background 0.15s ease;
}
.combo-box-header:hover {
  border-color: var(--fluent-accent, #0078d4);
}
.win-combo-box.is-disabled .combo-box-header {
  cursor: not-allowed;
  opacity: 0.6;
}
.combo-box-display {
  flex: 1;
  font-size: var(--fluent-font-size, 14px);
  color: var(--text-color, #323130);
}
.combo-box-placeholder {
  color: var(--fluent-muted, #6e6e6e);
}
.combo-box-glyph {
  color: var(--fluent-muted, #6e6e6e);
  transition: transform 0.15s ease;
}
.combo-box-glyph.is-flipped {
  transform: rotate(180deg);
}
.combo-box-popup {
  position: absolute;
  z-index: 1000;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: var(--card-bg, #fff);
  border: 1px solid var(--fluent-accent, #0078d4);
  border-radius: 4px;
  box-shadow: 0 8px 24px rgba(0, 0,JF, 0.18);
  box-shadow: 0 8px ＿＿24px rgba(0, 0, 0, 0.18);
  overflow: hidden;
}
.combo-box-filter {
  padding: 8px 8px 4px;
  border-bottom: 1px solid var(--fluent-border, #d1d1d1);
}
.combo-box-list {
  max-height: 260px;
  overflow-y: auto;
}
.combo-box-item {
  padding: 8px 12px;
  cursor: pointer;
  font-size: var(--fluent-font-size, 14px);
  color: var(--text-color, #323130);
  transition: background 0.12s ease;
}
.combo-box-item:hover,
.combo-box-item.is-active {
  background: var(--fluent-accent-light, rgba(0, 120, 212, 0.12));
}
.combo-box-item.is-selected {
  color: var(--fluent-accent, #0078d4);
  font-weight: 600;
}
.combo-box-error {
  margin-top: 4px;
  font-size: 12px;
  color: #d13438;
}
.combo-box-popup-enter-active,
.combo-box-popup-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.combo-box-popup-enter-from,
.combo-box-popup-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
