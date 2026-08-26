export interface ContextMenuPosition { x: number; y: number }

export interface ContextMenuItem {
  id: string; label?: string; icon?: any; disabled?: boolean;
  separator?: boolean; children?: ContextMenuItem[];
  onClick?: (item: ContextMenuItem, event: MouseEvent) => void; className?: string;
}

export interface ContextMenuProps {
  visible: boolean; position: ContextMenuPosition; items: ContextMenuItem[];
  className?: string; width?: number; maxHeight?: number; zIndex?: number;
  onClose?: () => void;
  onItemClick?: (item: ContextMenuItem, event: MouseEvent) => void;
}
