// Все вызовы бэкенда в одном месте. Имена и аргументы совпадают с src-tauri/src/commands.rs.
import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
    version: string;
    commit: string;
    buildDate: string;
    os: "windows" | "linux" | string;
    arch: string;
    debug: boolean;
}

export interface Settings {
    autoStart: boolean;
    notifications: boolean;
    minimizeToTray: boolean;
    gameFilter: boolean;
    refreshBridges: boolean;
    autoTor: boolean;
}

export interface FileNode {
    name: string;
    is_dir: boolean;
    children?: FileNode[];
    path: string;
}

export interface HostsData {
    date: string;
    categories: Record<string, string[]>;
}

export type AddType = "strategy" | "ipset" | "hostlist";

export const api = {
    appInfo: () => invoke<AppInfo>("get_app_info"),
    log: (text: string) => invoke<void>("log", { text }).catch(() => { }),
    openLink: (url: string) => invoke<void>("open_link", { url }),
    openFolder: (kind: "strategies" | "ipset-configs" | "lists" | "") => invoke<void>("open_folder", { kind }),
    openInEditor: (path: string) => invoke<void>("open_in_editor", { path }),

    loadSettings: () => invoke<Settings>("load_settings"),
    saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),

    // zapret
    strategies: () => invoke<string[]>("get_list_strategies"),
    customIpsets: () => invoke<string[]>("get_custom_configs"),
    currentStrategy: () => invoke<string | null>("get_strategy"),
    zapretActive: () => invoke<boolean>("is_active"),
    startZapret: (strategy: string, ipset: string | null) => invoke<boolean>("start_service", { strategy, ipset }),
    stopZapret: () => invoke<void>("stop_service"),
    syncZapretFiles: () => invoke<void>("sync_zapret_files"),
    restoreZapretFiles: () => invoke<string>("restore_zapret_files"),
    convertBats: (paths: string[]) => invoke<number>("convert_multiple_bats", { paths }),
    checkStrategyUpdates: () => invoke<string[]>("check_strategy_updates"),
    applyStrategyUpdate: (fileName: string) => invoke<void>("apply_strategy_update", { fileName }),
    updateTlsBin: () => invoke<string>("update_tls_bin"),
    repairZapretForTor: (stratName: string) => invoke<void>("zapret_4_tor", { stratName }),
    fileTree: () => invoke<FileNode[]>("get_file_tree"),
    missingResources: (strategy: string | null, ipset: string | null) =>
        invoke<string[]>("check_resources_exist", { strategy, ipset }),
    addFromLink: (addType: AddType, url: string, customName?: string) =>
        invoke<string>("handle_add_link", { addType, url, customName }),

    // hosts / DNS
    hostsData: () => invoke<HostsData>("get_hosts_data"),
    saveHosts: (selectedLines: string) => invoke<void>("save_hosts_selection", { selectedLines }),
    installedHostsCategories: () => invoke<string[]>("get_installed_categories"),
    flushDns: () => invoke<string>("flush_dns"),

    // прокси для Telegram
    proxyList: () => invoke<string[]>("get_proxy_list"),
    proxyPing: (address: string) => invoke<{ ping: number; country_code: string } | null>("check_proxy_ping", { address }),

    // TOR
    startTor: () => invoke<void>("start_tor"),
    stopTor: () => invoke<void>("stop_tor"),
    torStatus: () => invoke<{ running: boolean; progress: number }>("check_tor_status"),
    setSystemProxy: (enable: boolean) => invoke<void>(enable ? "enable_system_proxy" : "disable_system_proxy"),

    // точка доступа
    startHotspot: (port: number | null) => invoke<number>("start_hotspot", { port }),
    stopHotspot: () => invoke<void>("stop_hotspot"),
    hotspotPort: () => invoke<number | null>("get_hotspot_status"),
};

export const log = api.log;

/** Сравнение версий "2.1.0" > "2.0.10". Суффиксы (-beta) игнорируются. */
export function isNewerVersion(latest: string, current: string): boolean {
    const parse = (v: string) => v.trim().replace(/^v/, "").split("-")[0].split(".").map((n) => parseInt(n, 10) || 0);
    const [l, c] = [parse(latest), parse(current)];
    for (let i = 0; i < Math.max(l.length, c.length); i++) {
        if ((l[i] ?? 0) !== (c[i] ?? 0)) return (l[i] ?? 0) > (c[i] ?? 0);
    }
    return false;
}

export const stripExt = (name: string) => name.replace(/\.zapret$/i, "");
