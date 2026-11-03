import { useCallback, useEffect, useRef, useState } from "react";
import { notify } from './Notifications';
import { api, log } from "./lib/api";

export type Status = 'stopped' | 'running' | 'loading';
const DEFAULT_IPSET = "none";
const POLL_MS = 3000;

/** Состояние zapret: выбранная/запущенная стратегия, ipset, запуск и остановка. */
export function useZapret() {
    const [status, setStatus] = useState<Status>('loading');
    const [running, setRunning] = useState<string | null>(null);
    const [configs, setConfigs] = useState<string[]>([]);
    const [strategy, setStrategyState] = useState(() => localStorage.getItem("selected_strategy") || "");
    const [ipset, setIpsetState] = useState(() => localStorage.getItem("selected_ipset") || DEFAULT_IPSET);

    // актуальные значения для асинхронных функций (без устаревших замыканий)
    const ref = useRef({ strategy, ipset, busy: false });
    ref.current.strategy = strategy;
    ref.current.ipset = ipset;

    const setStrategy = useCallback((name: string) => {
        ref.current.strategy = name;
        setStrategyState(name);
        localStorage.setItem("selected_strategy", name);
    }, []);

    const setIpset = useCallback((value: string) => {
        ref.current.ipset = value;
        setIpsetState(value);
        localStorage.setItem("selected_ipset", value);
    }, []);

    /** Спрашивает у бэкенда, работает ли zapret и с какой стратегией. */
    const refresh = useCallback(async () => {
        const active = await api.zapretActive();
        const name = active ? await api.currentStrategy() : null;
        setRunning(name);
        setStatus(active ? 'running' : 'stopped');
        return active;
    }, []);

    const refreshList = useCallback(async () => {
        const list = await api.strategies();
        setConfigs(list);
        return list;
    }, []);

    const start = useCallback(async (forcedStrategy?: string, forcedIpset?: string) => {
        const name = forcedStrategy || ref.current.strategy;
        if (!name) {
            notify("Сначала выберите стратегию", "info");
            return;
        }
        ref.current.busy = true;
        setStatus('loading');
        try {
            const ok = await api.startZapret(name, forcedIpset || ref.current.ipset || DEFAULT_IPSET);
            if (!ok) notify("zapret не запустился — подробности во вкладке «Логи»", "error");
        } catch (e) {
            log("Ошибка запуска zapret: " + e);
            notify(`Ошибка запуска: ${e}`, "error");
        } finally {
            ref.current.busy = false;
            await refresh();
        }
    }, [refresh]);

    const stop = useCallback(async () => {
        ref.current.busy = true;
        setStatus('loading');
        try {
            await api.stopZapret();
        } catch (e) {
            notify(`Ошибка при остановке: ${e}`, "error");
        } finally {
            ref.current.busy = false;
            await refresh();
        }
    }, [refresh]);

    const toggle = useCallback(() => (status === 'running' ? stop() : start()), [status, start, stop]);

    const init = useCallback(async () => {
        const list = await refreshList();
        const active = await refresh();
        const current = active ? await api.currentStrategy() : null;
        if (current) {
            setStrategy(current);
            const runningIpset = localStorage.getItem("selected_ipset");
            if (runningIpset) setIpsetState(runningIpset);
        } else if (ref.current.strategy && !list.includes(ref.current.strategy)) {
            setStrategy(list[0] ?? "");
        } else if (!ref.current.strategy && list.length) {
            setStrategy(list[0]);
        }
    }, [refresh, refreshList, setStrategy]);

    // zapret может остановиться извне (служба, другой инструмент) — периодически сверяемся
    useEffect(() => {
        if (status === 'loading') return;
        const id = setInterval(() => {
            if (!ref.current.busy) refresh().catch(() => { });
        }, POLL_MS);
        return () => clearInterval(id);
    }, [status, refresh]);

    return {
        status, running, configs, strategy, ipset,
        setStrategy, setIpset, refreshList, refresh,
        start, stop, toggle, init,
    };
}

export type Zapret = ReturnType<typeof useZapret>;
