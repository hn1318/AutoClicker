import { reactive } from "vue";

/**
 * A single custom context menu shared by the whole app.
 *
 * The WebView2 default menu (refresh / save as / print ...) is suppressed
 * globally; only controls that can actually do something get a menu.
 */
export interface ContextMenuItem {
  id: string;
  label: string;
  disabled?: boolean;
  run: () => void | Promise<void>;
}

export interface ContextMenuState {
  open: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
}

const state = reactive<ContextMenuState>({
  open: false,
  x: 0,
  y: 0,
  items: [],
});

export function useContextMenu() {
  function show(x: number, y: number, items: ContextMenuItem[]): void {
    if (items.length === 0) {
      hide();
      return;
    }
    state.items = items;
    state.x = x;
    state.y = y;
    state.open = true;
  }

  function hide(): void {
    state.open = false;
    state.items = [];
  }

  return { state, show, hide };
}
