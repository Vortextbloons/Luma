import { native, onPluginWidgets, pluginWidgetsData } from '../services/tauri';
import type { PluginWidgetData } from '../services/plugins';

class PluginWidgetsStore {
  items = $state.raw<PluginWidgetData[]>([]);

  async start() {
    if (!native) return () => {};
    let active = true;
    void pluginWidgetsData().then(list => { if (active) this.items = list; }).catch(() => {});
    const stop = await onPluginWidgets(list => {
      if (active) this.items = list;
    });
    return () => {
      active = false;
      stop();
    };
  }

  forType(type: string) {
    return this.items.find(item => item.type === type) ?? null;
  }

  available(type: string) {
    const item = this.forType(type);
    return item ? item.available !== false : true;
  }
}

export const pluginWidgets = new PluginWidgetsStore();
