<template>
  <ContextMenu v-model:visible="visible" :position="position" :items="items" @close="handleClose" />
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import ContextMenu from './ContextMenu.vue'
import { type ContextMenuItem, type ContextMenuPosition } from './types'
import { CutIcon, CopyIcon, PasteIcon, CheckDoubleIcon } from 'tdesign-icons-vue-next'

const { t } = useI18n()
const visible = ref(false)
const position = ref<ContextMenuPosition>({ x: 0, y: 0 })
const items = ref<ContextMenuItem[]>([])
let targetElement: HTMLElement | null = null

const handleClose = () => { visible.value = false; targetElement = null }
const restoreFocus = () => { if (targetElement) targetElement.focus() }

const handleContextMenu = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable
  if (isInput && !target.getAttribute('disabled')) {
    e.preventDefault()
    targetElement = target
    const inputElement = target as HTMLInputElement | HTMLTextAreaElement
    const isReadonly = target.getAttribute('readonly') !== null
    let selection = ''
    if (window.getSelection()?.toString()) selection = window.getSelection()!.toString()
    else if (inputElement.value && typeof inputElement.selectionStart === 'number') {
      selection = inputElement.value.substring(inputElement.selectionStart, inputElement.selectionEnd || 0)
    }
    const hasSelection = selection.length > 0
    items.value = [
      { id: 'cut', label: t('common.cut'), icon: CutIcon, disabled: isReadonly || !hasSelection, onClick: () => { restoreFocus(); document.execCommand('cut') } },
      { id: 'copy', label: t('common.copy'), icon: CopyIcon, disabled: !hasSelection, onClick: () => { restoreFocus(); document.execCommand('copy') } },
      { id: 'paste', label: t('common.paste'), icon: PasteIcon, disabled: isReadonly, onClick: async () => {
        restoreFocus()
        try {
          const text = await navigator.clipboard.readText()
          if (text && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) {
            const input = target as HTMLInputElement
            input.focus()
            document.execCommand('insertText', false, text)
          }
        } catch {}
      }},
      { id: 'separator', separator: true },
      { id: 'select-all', label: t('common.selectAll'), icon: CheckDoubleIcon, onClick: () => {
        restoreFocus()
        if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') (target as HTMLInputElement).select()
        else document.execCommand('selectAll')
      }}
    ]
    position.value = { x: e.clientX, y: e.clientY }
    visible.value = true
  }
}

onMounted(() => window.addEventListener('contextmenu', handleContextMenu))
onUnmounted(() => window.removeEventListener('contextmenu', handleContextMenu))
</script>
