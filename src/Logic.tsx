import { useCallback, useEffect, useState } from "react";
import { emit, listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { notify } from './Notifications';
import { useZapret } from "./ZapretUtils";
import { api, isNewerVersion, log, type AppInfo } from "./lib/api";
import type { DeepLinkData } from "./modals/DeepLinkModal";

export { log };

const VERSION_URL = "https://raw.githubusercontent.com/f1zzen/zust/main/version";
const MAX_LOG_LINES = 300;

export type PageId = 'home' | 'settings' | 'proxy' | 'editor' | 'logs' | 'credits';
export type ModalId =
    | 'strategy' | 'convert' | 'ipset' | 'hosts' | 'proxy' | 'news'
    | 'deeplink' | 'hotspot' | 'sponsors' | 'community';

export function useApp() {
    const zapret = useZapret();
    const [activePage, setActivePageState] = useState<PageId>('home');
    const [modal, setModal] = useState<ModalId | null>(null);
    const [isPinned, setIsPinned] = useState(() => localStorage.getItem('window_pinned') === 'true');
    const [logs, setLogs] = useState<string[]>([]);
    const [torLogs, setTorLogs] = useState<string[]>([]);
    const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
    const [updateAvailable, setUpdateAvailable] = useState<string | null>(null);
    const [updatableStrats, setUpdatableStrats] = useState<string[]>([]);
    const [customIpsetFiles, setCustomIpsetFiles] = useState<string[]>([]);
    const [ipsetView, setIpsetView] = useState<'main' | 'custom'>('main');
    const [hoveredDesc, setHoveredDesc] = useState<string | null>(null);
    const [deepLinkData, setDeepLinkData] = useState<DeepLinkData | null>(null);
    const [hotspotPort, setHotspotPort] = useState<number | null>(null);

    const setActivePage = useCallback((page: PageId) => {
        setActivePageState(page);
        window.dispatchEvent(new CustomEvent('page-switch', { detail: page }));
    }, []);

    const openModal = useCallback((id: ModalId) => setModal(id), []);
    const closeModal = useCallback(() => setModal(null), []);

    const checkUpdate = async (currentVersion: string) => {
        try {
            const res = await fetch(`${VERSION_URL}?t=${Date.now()}`, { cache: 'no-store' });
            const latest = res.ok ? (await res.text()).trim() : null;
            if (latest && isNewerVersion(latest, currentVersion)) {
                setUpdateAvailable(latest);
                notify(`Доступна новая версия: v${latest}`, "info");
            }
        } catch (e) {
            log("Ошибка проверки версии: " + e);
        } finally {
            // новости сами закроются, если их нет или они уже прочитаны
            setModal((m) => m ?? 'news');
        }
    };

    useEffect(() => {
        const win = getCurrentWindow();
        if (localStorage.getItem('window_pinned') === 'true') {
            win.setAlwaysOnTop(true);
        }

        const initialize = async () => {
            const info = await api.appInfo();
            setAppInfo(info);
            try {
                await api.syncZapretFiles();
            } catch (e) {
                log("Ошибка синхронизации файлов zapret: " + e);
            }
            await Promise.all([
                zapret.init().catch((e) => log("zapret init: " + e)),
                api.checkStrategyUpdates().then(setUpdatableStrats).catch((e) => log("Проверка стратегий: " + e)),
                api.hotspotPort().then(setHotspotPort).catch(() => { }),
            ]);
            await emit("app_ready");

            checkUpdate(info.version);
            // фоновое обновление .bin — интерфейс его не ждёт.
            // Движок (winws/nfqws) из интернета не обновляется: только вместе с релизом Zust.
            api.updateTlsBin().then(log).catch((e) => log("Обновление bin-файлов: " + e));
        };
        initialize();

        const unlistenDeepLink = listen<{ action: string, params: Record<string, string> }>('zust-deeplink', async ({ payload }) => {
            const { action, params } = payload;
            switch (action) {
                case 'restore_zapret':
                    setDeepLinkData({ type: 'restore_zapret', payload: 'all_files' });
                    setModal('deeplink');
                    break;
                case 'flush_dns':
                    try {
                        log(await api.flushDns());
                        notify("Кэш DNS очищен", "success");
                    } catch (e) {
                        notify(`Ошибка очистки DNS: ${e}`, "error");
                    }
                    break;
                case 'add': {
                    const type = params.type?.toLowerCase();
                    if (!params.url || !['strategy', 'ipset', 'hostlist'].includes(type)) break;
                    const fileName = params.url.split('/').pop()?.split('.')[0] || 'imported_file';
                    setDeepLinkData({
                        type: type as DeepLinkData['type'],
                        payload: params.url,
                        name: type === 'hostlist' ? 'list-general.txt' : (params.name || fileName),
                    });
                    setModal('deeplink');
                    break;
                }
                case 'configure_zapret': {
                    const { strategy, ipset_config, game_filter, restart } = params;
                    setDeepLinkData({
                        type: 'configure_zapret',
                        payload: JSON.stringify({ strategy, ipset_config, game_filter, restart }),
                        name: 'Глубокая настройка',
                    });
                    setModal('deeplink');
                    break;
                }
                // пасхалки
                case 'milana_hametova':
                    api.openLink('https://i.imgur.com/CGqXjZB.png');
                    break;
                case 'net':
                    api.openLink('https://i.imgur.com/GuuLZta.png');
                    break;
                default:
                    log(`Команда ${action} не распознана`);
            }
        });

        const unlistenLog = listen<string>('log-event', ({ payload }) => {
            const add = (prev: string[]) => [payload, ...prev.slice(0, MAX_LOG_LINES - 1)];
            if (payload.includes('[TOR]')) setTorLogs(add);
            else setLogs(add);
        });

        const blockContextMenu = (e: MouseEvent) => e.preventDefault();
        window.addEventListener('contextmenu', blockContextMenu);

        if (import.meta.env.DEV) {
            (window as any).testNewsModal = () => {
                localStorage.removeItem("last_seen_news_id");
                setModal('news');
            };
        }

        return () => {
            unlistenDeepLink.then((f) => f());
            unlistenLog.then((f) => f());
            window.removeEventListener('contextmenu', blockContextMenu);
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    const actions = {
        changeIpsetMode: async (mode: 'Any' | 'None' | 'Custom', fileName?: string) => {
            const value = mode === 'Any' ? 'any' : mode === 'None' ? 'none' : (fileName || 'any');
            zapret.setIpset(value);
            if (zapret.status === 'running') await zapret.start(undefined, value);
        },
        pickBatFiles: async () => {
            const selected = await open({ multiple: true, filters: [{ name: 'Batch', extensions: ['bat'] }] });
            if (!selected) return;
            try {
                const count = await api.convertBats(Array.isArray(selected) ? selected : [selected]);
                closeModal();
                await zapret.refreshList();
                notify(count ? `Сконвертировано: ${count}` : "В файлах не найден запуск winws.exe", count ? "success" : "error");
            } catch (e) {
                notify(`Ошибка при конвертации: ${e}`, "error");
            }
        },
        selectStrategy: async (name: string) => {
            closeModal();
            zapret.setStrategy(name);
            await zapret.start(name);
        },
        togglePin: async () => {
            const next = !isPinned;
            setIsPinned(next);
            await getCurrentWindow().setAlwaysOnTop(next);
            localStorage.setItem('window_pinned', String(next));
        },
        loadIpsetConfigs: async () => setCustomIpsetFiles(await api.customIpsets()),
    };

    return {
        zapret, activePage, setActivePage, modal, openModal, closeModal,
        isPinned, logs, torLogs, appInfo, updateAvailable,
        updatableStrats, setUpdatableStrats, customIpsetFiles, ipsetView, setIpsetView,
        hoveredDesc, setHoveredDesc, deepLinkData, hotspotPort, setHotspotPort,
        actions,
    };
}

export type AppState = ReturnType<typeof useApp>;
