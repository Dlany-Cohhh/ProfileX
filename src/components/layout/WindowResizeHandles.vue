<template>
  <!-- 无边框窗口没有系统边框，这里补上 8 个缩放热区 -->
  <div class="window-resize" aria-hidden="true">
    <div
      v-for="edge in edges"
      :key="edge.dir"
      :class="['rz', `rz-${edge.cls}`]"
      @mousedown="start(edge.dir)"
    />
  </div>
</template>

<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';

type Dir =
  | 'North'
  | 'South'
  | 'East'
  | 'West'
  | 'NorthEast'
  | 'NorthWest'
  | 'SouthEast'
  | 'SouthWest';

const edges: { dir: Dir; cls: string }[] = [
  { dir: 'North', cls: 'n' },
  { dir: 'South', cls: 's' },
  { dir: 'East', cls: 'e' },
  { dir: 'West', cls: 'w' },
  { dir: 'NorthEast', cls: 'ne' },
  { dir: 'NorthWest', cls: 'nw' },
  { dir: 'SouthEast', cls: 'se' },
  { dir: 'SouthWest', cls: 'sw' },
];

const appWindow = getCurrentWindow();

const start = (direction: Dir) => {
  appWindow.startResizeDragging(direction);
};
</script>

<style scoped lang="scss">
.window-resize {
  position: fixed;
  inset: 0;
  z-index: 9998;
  pointer-events: none;

  .rz {
    position: absolute;
    pointer-events: auto;
  }

  .rz-n {
    top: 0;
    left: 0;
    right: 0;
    height: 5px;
    cursor: ns-resize;
  }

  .rz-s {
    bottom: 0;
    left: 0;
    right: 0;
    height: 5px;
    cursor: ns-resize;
  }

  .rz-w {
    top: 0;
    bottom: 0;
    left: 0;
    width: 5px;
    cursor: ew-resize;
  }

  .rz-e {
    top: 0;
    bottom: 0;
    right: 0;
    width: 5px;
    cursor: ew-resize;
  }

  .rz-nw,
  .rz-ne,
  .rz-sw,
  .rz-se {
    width: 10px;
    height: 10px;
  }

  .rz-nw {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }

  .rz-ne {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }

  .rz-sw {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }

  .rz-se {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
  }
}
</style>
