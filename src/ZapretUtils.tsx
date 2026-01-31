import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { notify } from './Notifications';

export type Status = 'stopped' | 'running' | 'loading';
const DEFAULT_IPSET = "none";

export function ZapretUtils() {
    const [status, setStatus] = useState<Status>('loading');
    const [stratName, setStratName] = useState('Проверяем...');
    const [configs, setConfigs] = useState<string[]>([]);
    const [selectedConfig, setSelectedConfig] = useState("");
    const [selectedIpset, setSelectedIpset] = useState<string | null>(null);

    const checkZapret = async () => {
        const name = await invoke<string>('get_strategy');
        setStratName(name);
        setStatus(name !== "Отсутствует" ? 'running' : 'stopped');
    };

    const stopProcess = async () => {
        setStatus('loading');
        try {
            await invoke('stop_service');
            await checkZapret();
        } catch {
            notify("Ошибка при остановке", "error");
            setStatus('stopped');
        }
    };

    const startProcess = async (forcedConfig?: string, forcedIpset?: string) => {
        setStatus('loading');
        const configName = forcedConfig || selectedConfig || localStorage.getItem("selected_strategy") || "";
        const ipsetConfig = forcedIpset || selectedIpset || localStorage.getItem("selected_ipset") || DEFAULT_IPSET;
        const idx = configs.indexOf(configName) + 1;
        if (idx <= 0) {
            console.error("Config not found:", configName);
            return setStatus('stopped');
        }

        try {
            await invoke('start_service', { args: { index: idx, ipset_config: ipsetConfig } });
            setTimeout(checkZapret, 500);
        } catch (e) {
            console.error(e);
            setStatus('stopped');
        }
    };

    const init = async () => {
        const list = await invoke<string[]>('get_list_strategies');
        setConfigs(list);

        const currentRunning = await invoke<string>('get_strategy');
        const savedIpset = localStorage.getItem("selected_ipset") || DEFAULT_IPSET;
        setSelectedIpset(savedIpset);

        if (currentRunning !== "Отсутствует") {
            setStratName(currentRunning);
            setSelectedConfig(currentRunning);
            setStatus('running');
        } else {
            const savedConfig = localStorage.getItem("selected_strategy");
            if (savedConfig && list.includes(savedConfig)) {
                setSelectedConfig(savedConfig);
            }
            setStatus('stopped');
        }
        return { list, currentRunning };
    };

    return {
        status, stratName, configs, selectedConfig, selectedIpset,
        setConfigs, setSelectedConfig, setSelectedIpset,
        startProcess, stopProcess, checkZapret, init
    };
}