import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { log } from '../Logic';
import { ActionButton, DropDownItemStyle, MenuButton } from '../Buttons';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/animate-ui/primitives/animate/tooltip';
import { AnimatePresence, motion } from 'motion/react';
import { cn } from '@/lib/utils';
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/animate-ui/components/radix/dropdown-menu';
import { notify } from '@/Notifications';
import { X, Loader2, Sparkles } from 'lucide-react';

const STATUS = {
    stopped: 'Выключен.',
    loading: 'Подключение:',
    running: 'Подключение обеспечено.'
};

const LOADING_PHRASES = [
    "Долговато..",
    "Перепроверяю..",
    "Обходим ограничения..",
    "Почти готово.."
];

const MODES = [
    { id: 'vpn', label: 'Системный', tooltip: 'Работает системно, обрабатывает ваши запросы через прокси.' },
    { id: 'direct', label: 'Фоновый', tooltip: 'Работает в фоне, не вмешиваясь в работу приложений.' },
];

export const ProxyPage = ({
    isHotspotActive,
    setIsHotspotOpen,
    activeHotspotPort,
    setIsHotspotActive,
    setActiveHotspotPort
}: any) => {
    const [status, setStatus] = useState<'stopped' | 'loading' | 'running'>('stopped');
    const [fillLevel, setFillLevel] = useState(0);
    const [lastLog, setLastLog] = useState('Готовимся к запуску!');
    const [mode, setMode] = useState<'vpn' | 'direct'>('direct');
    const [isPending, setIsPending] = useState(false);

    useEffect(() => {
        const checkInitialStatus = async () => {
            try {
                const isRunning = await invoke<boolean>('check_tor_status');
                if (isRunning) {
                    setStatus('running');
                    setFillLevel(100);
                }
                const hotspotPort = await invoke<number | null>('get_hotspot_status');
                if (hotspotPort) {
                    setIsHotspotActive(true);
                    setActiveHotspotPort(hotspotPort);
                }
            } catch (err) {
                log("Запуск прокси/точки доступа вызвала ошибку: " + err);
            }
        };

        const unlistenProgress = listen<number>('tor-progress', (event) => {
            const progress = event.payload;
            setFillLevel(progress);
            if (progress === 100) setStatus('running');
        });

        const unlistenLog = listen<string>('log-event', (event) => {
            const rawLog = event.payload;
            if (rawLog.toLowerCase().includes("torrc is empty") || rawLog.includes("torrc empty")) {
                setLastLog("Файл конфигурации пуст!");
                setStatus('stopped');
                return;
            }

            if (rawLog.includes('[TOR]')) {
                let cleanLog = rawLog.replace('[TOR]', '').trim();
                if (cleanLog.includes("general SOCKS server failure")) return;
                const msg = cleanLog.replace(/^[A-Z][a-z]{2}\s+\d+\s+\d{2}:\d{2}:\d{2}\.\d{3}\s+\[\w+\]\s+/, '');
                setLastLog(msg);
            }
        });

        checkInitialStatus();

        return () => {
            unlistenProgress.then(f => f());
            unlistenLog.then(f => f());
        };
    }, []);

    const handleModeChange = async (newMode: 'vpn' | 'direct') => {
        setMode(newMode);
        if (status === 'running') {
            if (newMode === 'vpn') {
                await invoke('enable_system_proxy');
                setLastLog('Включён системный режим!');
            } else {
                await invoke('disable_system_proxy');
                setLastLog('Включён фоновый режим!');
            }
        }
    };

    const handleToggle = async () => {
        if (isPending && status !== 'loading') return;

        setIsPending(true);

        if (status === 'stopped') {
            setStatus('loading');
            setFillLevel(0);
            setLastLog('Инициализация...');

            try {
                await invoke('start_tor');
                if (mode === 'vpn') {
                    await invoke('enable_system_proxy');
                }
            } catch (err) {
                if (err !== 'interrupted') {
                    setLastLog(`Ошибка: ${err}`);
                    setStatus('stopped');
                    await invoke('stop_tor');
                }
            } finally {
                setIsPending(false);
            }
        } else {

            setStatus('stopped');
            setFillLevel(0);
            setLastLog('Остановка...');

            try {
                await invoke('stop_tor');
                setLastLog('Готов к работе');
            } finally {
                setTimeout(() => setIsPending(false), 1000);
            }
        }
    };

    return (
        <div className="content">
            <div className="strategy-header">
                <span className="strat-label">Соединение</span>
                <div className="strat-title-row">
                    <div className="strat-value">Прокси TOR</div>
                </div>
            </div>

            <div className="mode-toggle-container">
                <TooltipProvider openDelay={0} closeDelay={100}>
                    {MODES.map((tab) => {
                        const isActive = mode === tab.id;
                        return (
                            <Tooltip key={tab.id}>
                                <TooltipTrigger asChild>
                                    <button
                                        className={cn(
                                            "relative flex-1 px-6 py-2 text-sm font-medium transition-colors duration-300",
                                            isActive ? "text-white" : "text-white/60 hover:text-white"
                                        )}
                                        onClick={() => (tab.id === 'vpn' ? !isPending && handleModeChange('vpn') : handleModeChange('direct'))}
                                    >
                                        {isActive && (
                                            <motion.div
                                                layoutId="mode-highlight"
                                                className="absolute inset-0 bg-purple-500/20 border border-purple-400/25 rounded-lg"
                                                transition={{
                                                    type: "tween",
                                                    ease: [0.4, 0, 0.2, 1],
                                                    duration: 0.3
                                                }}
                                            />
                                        )}
                                        <span className="relative z-10">{tab.label}</span>
                                    </button>
                                </TooltipTrigger>
                                <TooltipContent
                                    className="bg-[#05010a] border border-purple-500/30 text-white px-3 py-1.5 text-sm rounded-md backdrop-blur-md"
                                >
                                    <p>{tab.tooltip}</p>
                                </TooltipContent>
                            </Tooltip>
                        );
                    })}
                </TooltipProvider>
            </div>
            <ActionButton
                STATUS={{
                    ...STATUS,
                    loading: `Подключение: ${fillLevel}%`
                }}
                handleToggle={handleToggle}
                status={status}
            />

            <div className="action-buttons">
                <div className="separator"><span>Параметры</span></div>
                <div className={cn(
                    "relative w-full overflow-hidden transition-all duration-700 ease-[cubic-bezier(0.23,1,0.32,1)]",
                    "bg-[#05010a] border border-white/10 rounded-[2rem] shadow-none",
                    status === 'running' ? "h-20 border-purple-500/30" : "h-16" // Уменьшил высоту для компактности
                )}>
                    <div className="absolute inset-0 flex flex-col items-center justify-center px-8 text-center">
                        <AnimatePresence mode="wait">
                            {status === 'stopped' && (
                                <motion.div
                                    key="stopped"
                                    initial={{ y: 10, opacity: 0 }}
                                    animate={{ y: 0, opacity: 1 }}
                                    exit={{ y: -15, opacity: 0 }}
                                    className="flex items-center gap-2"
                                >
                                    <X className="w-4 h-4 text-white/40" />
                                    <span className={cn(
                                        "text-sm font-medium tracking-tight",
                                        lastLog === "Файл конфигурации пуст!" ? "text-red-400" : "text-white/80"
                                    )}>
                                        {lastLog === "Файл конфигурации пуст!" ? lastLog : "Система готова"}
                                    </span>
                                </motion.div>
                            )}

                            {status === 'loading' && (
                                <motion.div
                                    key="loading"
                                    initial={{ y: 15, opacity: 0, scale: 0.9 }}
                                    animate={{ y: 0, opacity: 1, scale: 1 }}
                                    exit={{ y: -15, opacity: 0 }}
                                    className="flex flex-col items-center gap-1"
                                >
                                    <div className="flex items-center gap-2">
                                        <Loader2 className="w-4 h-4 text-purple-400 animate-spin" />
                                        <span className="text-purple-400 text-sm font-semibold animate-pulse tracking-wide">
                                            {fillLevel < 30 ? "Подготовка..." : LOADING_PHRASES[Math.floor((fillLevel / 100) * LOADING_PHRASES.length)]}
                                        </span>
                                    </div>
                                    <span className="text-[10px] text-white/60 tracking-[0.2em]">{fillLevel}%</span>
                                </motion.div>
                            )}

                            {status === 'running' && (
                                <motion.div
                                    key="running"
                                    initial={{ y: 15, opacity: 0 }}
                                    animate={{ y: 0, opacity: 1 }}
                                    className="flex flex-col items-center w-full"
                                >
                                    <div className="flex items-center gap-2 mb-0.5">
                                        <Sparkles className="w-3.5 h-3.5 text-purple-500" />
                                        <span className="text-purple-500 text-sm font-bold">
                                            Соединение активно
                                        </span>
                                    </div>

                                    <TooltipProvider openDelay={300}>
                                        <Tooltip key={lastLog} side="bottom">
                                            <TooltipTrigger asChild>
                                                <motion.p
                                                    initial={{ opacity: 0 }}
                                                    animate={{ opacity: 1 }}
                                                    className="text-[11px] text-white/50 font-medium truncate max-w-[220px] cursor-help"
                                                >
                                                    {lastLog.includes('режим') ? "Прокси продолжает работать!" : lastLog.replace(/^\[\w+\]\s*/, '').trim()}
                                                </motion.p>
                                            </TooltipTrigger>
                                            <TooltipContent className="bg-[#05010a] translate-y-7 border border-purple-500/30 text-white px-3 py-1 text-sm rounded-md backdrop-blur-md">
                                                {lastLog.includes('режим') ? "Прокси продолжает работать!" : lastLog.replace(/^\[\w+\]\s*/, '').trim()}
                                            </TooltipContent>
                                        </Tooltip>
                                    </TooltipProvider>
                                </motion.div>
                            )}
                        </AnimatePresence>
                    </div>

                    {status === 'loading' && (
                        <motion.div
                            className="absolute bottom-0 left-0 h-[1.5px] bg-purple-600"
                            initial={{ width: 0 }}
                            animate={{ width: `${fillLevel}%` }}
                        />
                    )}
                </div>
                {status === 'running' && (
                    <>
                        <div className="separator"><span>Действия</span></div>
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <MenuButton
                                    title="Добавить в Telegram"
                                    subtitle="Обход действующих замедлений/блокировок Telegram."
                                    icon={<svg width="24px" height="24px" viewBox="0 0 192 192" xmlns="http://www.w3.org/2000/svg" fill="none"><path stroke="#3b82f6" strokeWidth="12" d="M23.073 88.132s65.458-26.782 88.16-36.212c8.702-3.772 38.215-15.843 38.215-15.843s13.621-5.28 12.486 7.544c-.379 5.281-3.406 23.764-6.433 43.756-4.54 28.291-9.459 59.221-9.459 59.221s-.756 8.676-7.188 10.185c-6.433 1.509-17.027-5.281-18.919-6.79-1.513-1.132-28.377-18.106-38.214-26.404-2.649-2.263-5.676-6.79.378-12.071 13.621-12.447 29.891-27.913 39.728-37.72 4.54-4.527 9.081-15.089-9.837-2.264-26.864 18.483-53.35 35.835-53.35 35.835s-6.053 3.772-17.404.377c-11.351-3.395-24.594-7.921-24.594-7.921s-9.08-5.659 6.433-11.693Z" /></svg>}
                                    variant="proxy"
                                />
                            </DropdownMenuTrigger>

                            <DropdownMenuContent
                                className="w-64 p-1.5 bg-[#0a0514]/90 border border-white/5 rounded-xl shadow-2xl backdrop-blur-md"
                                sideOffset={8}
                            >
                                <DropdownMenuLabel className="px-3 py-2 text-[11px] font-medium text-white/30 uppercase tracking-widest">
                                    Основные действия
                                </DropdownMenuLabel>

                                <DropdownMenuGroup className="space-y-1">
                                    <DropdownMenuItem
                                        onSelect={async () => await invoke("open_link", { url: "tg://socks?server=127.0.0.1&port=9050" })}
                                        className={DropDownItemStyle}
                                    >
                                        Открыть в ТГ
                                    </DropdownMenuItem>

                                    <DropdownMenuItem
                                        onSelect={async () => {
                                            await navigator.clipboard.writeText("tg://socks?server=127.0.0.1&port=9050");
                                        }}
                                        className={DropDownItemStyle}
                                    >
                                        Скопировать ссылку
                                    </DropdownMenuItem>
                                </DropdownMenuGroup>

                                <DropdownMenuSeparator className="h-px bg-white/5 my-1.5" />

                                <DropdownMenuItem
                                    className="flex items-center px-3 py-2 text-sm text-red-400/70 bg-transparent rounded-lg cursor-pointer outline-none transition-all data-[highlighted]:!bg-red-500/10 data-[highlighted]:!text-red-400 active:!scale-[0.96]"
                                >
                                    Отмена
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>

                        {!isHotspotActive ? (
                            <MenuButton
                                title="Точка доступа"
                                subtitle="Разрешить подключения из локальной сети."
                                icon={<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21.54 15H17a2 2 0 0 0-2 2v4.54" /><path d="M7 3.34V5a3 3 0 0 0 3 3a2 2 0 0 1 2 2c0 1.1.9 2 2 2a2 2 0 0 0 2-2c0-1.1.9-2 2-2h3.17" /><path d="M11 21.95V18a2 2 0 0 0-2-2a2 2 0 0 1-2-2v-1a2 2 0 0 0-2-2H2.05" /><circle cx="12" cy="12" r="10" /></svg>}
                                onClick={() => setIsHotspotOpen(true)}
                                variant="proxy"
                            />
                        ) : (
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <MenuButton
                                        title="Раздача активна"
                                        subtitle="Точка доступа включена!"
                                        icon={<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M5 12.55a11 11 0 0 1 14.08 0" /><path d="M1.42 9a16 16 0 0 1 21.16 0" /><path d="M8.53 16.11a6 6 0 0 1 6.95 0" /><line x1="12" y1="20" x2="12.01" y2="20" /></svg>}
                                        variant="proxy"
                                    />
                                </DropdownMenuTrigger>
                                <DropdownMenuContent className="w-64 p-1.5 bg-[#0a0514]/90 border border-white/5 rounded-xl shadow-2xl backdrop-blur-md" sideOffset={8}>
                                    <DropdownMenuLabel className="px-3 py-2 text-[11px] font-medium text-white/30 uppercase tracking-widest">
                                        Основные действия
                                    </DropdownMenuLabel>
                                    <DropdownMenuGroup className="space-y-1">
                                        <DropdownMenuItem
                                            onSelect={async () => {
                                                await navigator.clipboard.writeText(`https://t.me/socks?server=frp.freefrp.net&port=${activeHotspotPort}`);
                                                notify("Ссылка на точку скопирована!");
                                            }}
                                            className={DropDownItemStyle}
                                        >
                                            Скопировать ссылку
                                        </DropdownMenuItem>
                                        <DropdownMenuItem
                                            onSelect={async () => {
                                                await navigator.clipboard.writeText(activeHotspotPort?.toString() || "");
                                                notify(`Порт точки доступа скопирован`);
                                            }}
                                            className={DropDownItemStyle}
                                        >
                                            Скопировать порт
                                        </DropdownMenuItem>
                                        <DropdownMenuItem
                                            onSelect={async () => {
                                                await invoke("open_link", { url: `tg://socks?server=frp.freefrp.net&port=${activeHotspotPort}` });
                                                notify("Открываем телеграмм..");
                                            }}
                                            className={DropDownItemStyle}
                                        >
                                            Открыть в Telegram
                                        </DropdownMenuItem>
                                    </DropdownMenuGroup>
                                    <DropdownMenuSeparator className="h-px bg-white/5 my-1.5" />
                                    <DropdownMenuItem
                                        onSelect={async () => {
                                            try {
                                                await invoke("stop_hotspot");
                                                setIsHotspotActive(false);
                                                notify("Точка доступа остановлена");
                                            } catch (err) {
                                                notify(`Ошибка остановки: ${err}`);
                                            }
                                        }}
                                        className="flex items-center px-3 py-2 text-sm text-red-400/70 bg-transparent rounded-lg cursor-pointer outline-none transition-all data-[highlighted]:!bg-red-500/10 data-[highlighted]:!text-red-400 active:!scale-[0.96]"
                                    >
                                        Закрыть точку доступа
                                    </DropdownMenuItem>
                                </DropdownMenuContent>
                            </DropdownMenu>
                        )}
                    </>
                )}
            </div>
        </div>
    );
};
