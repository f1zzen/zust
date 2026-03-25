import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModalAnimation } from "./useModalAnimation";
import { DropDownItemStyle, ModalBlock, ModalContent, ModalFooter, ModalHeader, SelectButton } from "@/Buttons";
import { cn } from "@/lib/utils";
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/animate-ui/components/radix/dropdown-menu";
import { DropdownMenuItem } from "@/components/animate-ui/primitives/radix/dropdown-menu";
import React from "react";

const ProxyRow = React.forwardRef<HTMLDivElement, { p: any; onClick: (addr: string) => void }>(
    ({ p, onClick, ...props }, ref) => {
        const { onClick: radixOnClick, ...restProps } = props as any;
        return (
            <div
                ref={ref}
                {...restProps}
                onClick={(e) => {
                    radixOnClick?.(e);
                    onClick(p.address);
                }}
                className="group flex items-center gap-4 rounded-xl border border-transparent bg-white/[0.03] p-3 transition-all hover:bg-white/[0.06] hover:border-white/5 cursor-pointer active:scale-[0.99]"
            >
                <div className="flex w-6 shrink-0 justify-center">
                    {p.ping === 'loading' ? (
                        <span className="text-base animate-pulse">⏳</span>
                    ) : p.country && p.country !== "??" ? (
                        <img
                            src={`https://purecatamphetamine.github.io/country-flag-icons/3x2/${p.country.toUpperCase()}.svg`}
                            className="w-5 rounded-[2px] shadow-sm"
                        />
                    ) : (
                        <span className="text-base text-white/40">🌐</span>
                    )}
                </div>
                <div className={cn(
                    "h-1.5 w-1.5 shrink-0 rounded-full transition-colors duration-300",
                    p.ping === 'loading' ? "bg-purple-500" : (p.ping === null ? "bg-red-500" : "bg-green-500")
                )} />
                <span className="flex-1 truncate font-['JetBrains_Mono'] text-[13px] text-white/70 group-hover:text-white transition-colors">
                    {p.address}
                </span>
                <div className="shrink-0 text-right min-w-[65px]">
                    {p.ping === 'loading' ? (
                        <span className="inline-block animate-spin text-purple-400 text-xs">↻</span>
                    ) : (
                        <span className={cn(
                            "text-[11px] font-bold tracking-tight",
                            p.ping === null ? "text-red-400/80" : "text-green-400"
                        )}>
                            {p.ping === null ? 'OFFLINE' : `${p.ping}ms`}
                        </span>
                    )}
                </div>
            </div>
        );
    }
);

ProxyRow.displayName = "ProxyRow";

export const ProxyModal = ({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) => {
    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);
    const [proxies, setProxies] = useState<any[]>([]);
    const [loading, setLoading] = useState(true);

    const loadAndPing = async () => {
        setLoading(true);
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 8000);

        try {
            const [list] = await Promise.all([
                invoke<string[]>("get_proxy_list"),
                new Promise(resolve => setTimeout(resolve, 600))
            ]);

            clearTimeout(timeoutId);
            setProxies(list.map(addr => ({ address: addr, ping: 'loading' })));

            list.forEach(async (addr) => {
                try {
                    const result = await invoke<any>("check_proxy_ping", { address: addr });
                    setProxies(prev => prev.map(p =>
                        p.address === addr ? {
                            ...p,
                            ping: result ? result.ping : null,
                            country: result ? result.country_code : "??"
                        } : p
                    ));
                } catch (e) {
                    setProxies(prev => prev.map(p =>
                        p.address === addr ? { ...p, ping: null, country: "??" } : p
                    ));
                }
            });
        } catch (e: any) {
            if (e.name === 'AbortError') console.error("Timeout");
            if (proxies.length === 0) onClose();
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => { if (isOpen) loadAndPing(); }, [isOpen]);

    if (!shouldRender) return null;

    return (
        <div
            className={cn(
                "fixed inset-0 z-[1000] flex items-center justify-center bg-black/60 backdrop-blur-md transition-opacity duration-300",
                isAnimatingOut ? "opacity-0" : "opacity-100"
            )}
            onClick={onClose}
        >
            <ModalBlock
                onClick={(e: React.MouseEvent) => e.stopPropagation()}
            >
                <ModalHeader
                    title="Прокси для Telegram"
                    status={proxies.length + " доступно"}
                    icon="🛫"
                    description="Используйте MTProto для стабильного соединения"
                />

                <ModalContent>
                    {loading ? (
                        <div className="flex h-44 flex-col items-center justify-center gap-3 animate-pulse text-white/30 text-sm font-['Onest']">
                            <span className="text-2xl">?</span>
                            Получение списка прокси...
                        </div>
                    ) : (
                        <div>
                            {proxies.map((p, i) => {
                                const proxyUrl = p.address.startsWith('tg://')
                                    ? p.address
                                    : `tg://socks?server=${p.address.split(':')[0]}&port=${p.address.split(':')[1] || '1080'}`;

                                return (
                                    <DropdownMenu key={i}>
                                        <DropdownMenuTrigger asChild>
                                            <ProxyRow p={p} onClick={() => { }} />
                                        </DropdownMenuTrigger>
                                        <DropdownMenuContent
                                            className="w-64 p-1.5 bg-[#0a0514]/90 border border-white/5 rounded-xl shadow-2xl backdrop-blur-md z-[1101]"
                                            sideOffset={8}
                                        >
                                            <DropdownMenuGroup className="space-y-1">
                                                <DropdownMenuLabel className="px-3 py-2 text-[11px] font-medium text-white/30 uppercase tracking-widest">
                                                    Основные действия
                                                </DropdownMenuLabel>
                                                <DropdownMenuItem
                                                    onSelect={async () => await invoke("open_link", { url: proxyUrl })}
                                                    className={DropDownItemStyle}
                                                >
                                                    Открыть в ТГ
                                                </DropdownMenuItem>
                                                <DropdownMenuItem
                                                    onSelect={async () => await navigator.clipboard.writeText(proxyUrl)}
                                                    className={DropDownItemStyle}
                                                >
                                                    Скопировать ссылку
                                                </DropdownMenuItem>
                                            </DropdownMenuGroup>
                                            <DropdownMenuSeparator className="h-px bg-white/5 my-1.5" />
                                            <DropdownMenuItem className="flex items-center px-3 py-2 text-sm text-red-400/70 bg-transparent rounded-lg cursor-pointer outline-none transition-all data-[highlighted]:!bg-red-500/10 data-[highlighted]:!text-red-400 active:!scale-[0.96]">
                                                Отмена
                                            </DropdownMenuItem>
                                        </DropdownMenuContent>
                                    </DropdownMenu>
                                );
                            })}
                        </div>
                    )}
                </ModalContent>

                <ModalFooter>
                    <SelectButton onClick={loadAndPing} front="Обновить" back="Обновить" />
                    <button
                        className="rounded-lg bg-white/5 px-4 py-1 font-['Onest'] text-[13px] font-bold text-white/60 transition-all hover:bg-white/10 hover:text-white active:scale-95"
                        onClick={onClose}
                    >
                        Закрыть
                    </button>
                </ModalFooter>
            </ModalBlock>
        </div>
    );
};