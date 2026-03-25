import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useModalAnimation } from "./useModalAnimation";
import { InfoHeader, ModalBlock, ModalContent, ModalFooter, ModalHeader, SelectButton } from "@/Buttons";
import { cn } from "@/lib/utils";
import React from "react";
import { notify } from "@/Notifications";

export const HotspotModal = ({ isOpen, onClose, onLaunched }: {
    isOpen: boolean;
    onClose: () => void;
    onLaunched: (port: number) => void;
}) => {
    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);
    const [portInput, setPortInput] = useState<string>("");
    const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
    const [statusText, setStatusText] = useState<string>("Подготовка...");
    const [errorText, setErrorText] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(false);

    useEffect(() => {
        if (isOpen) {
            setStatus('idle');
            setPortInput("");
            setErrorText(null);
            setStatusText("Подготовка...");
        }
    }, [isOpen]);

    useEffect(() => {
        let unlisten: () => void;

        const initListener = async () => {
            const fn = await listen<string>("log-event", (event) => {
                setStatusText(event.payload);
                if (event.payload.toLowerCase().includes("error")) {
                    setErrorText(event.payload);
                }
            });
            unlisten = fn;
        };

        initListener();
        return () => { if (unlisten) unlisten(); };
    }, []);

    const handleStart = async () => {
        if (isLoading) return;

        setIsLoading(true);
        try {
            const port = await invoke<number>("start_hotspot", {
                port: Number(portInput) || 7291
            });

            onLaunched(port);
            onClose();
            notify("Точка доступа запущена!", "success");

        } catch (e) {
            notify(`Ошибка: ${e}`, "error");
        } finally {
            setIsLoading(false);
        }
    };

    if (!shouldRender) return null;

    return (
        <div
            className={cn(
                "fixed inset-0 z-[1000] flex items-center justify-center bg-black/60 backdrop-blur-md transition-opacity duration-300",
                isAnimatingOut ? "opacity-0" : "opacity-100"
            )}
            onClick={onClose}
        >
            <ModalBlock onClick={(e: React.MouseEvent) => e.stopPropagation()}>
                <ModalHeader
                    title="Точка доступа"
                    status={status === 'success' ? ":D" : "Конфигурация"}
                    icon="🛡️"
                    description="Приход локального SOCK5 прокси в глобальную сеть!"
                />

                <ModalContent>
                    <div className="space-y-4 py-2">
                        {status === 'success' ? (
                            <div className="flex h-32 flex-col items-center justify-center gap-3 text-green-400">
                                <div className="flex h-12 w-12 items-center justify-center rounded-full bg-green-500/20 text-2xl animate-bounce">✓</div>
                                <span className="text-sm font-medium">Запущено успешно!</span>
                            </div>
                        ) : (
                            <>
                                <InfoHeader description="Точка доступа используется для раздачи прокси (в основном) людям без Z.UST-а. Если вы хотите созвониться со своим другом с телефона, например, в Telegram - достаточно включить точку доступа и подключить прокси на обоих устройствах." more="Скорость может быть медленее, чем на ПК. Не стоит удивляться." />
                                <div className="flex flex-col gap-2">
                                    <label className="text-[11px] uppercase tracking-widest text-white/30 ml-1">Порт</label>
                                    <input
                                        type="number"
                                        value={portInput}
                                        onChange={(e) => setPortInput(e.target.value)}
                                        placeholder="Порт"
                                        disabled={status === 'loading'}
                                        className="w-full rounded-xl border border-white/5 bg-white/[0.03] p-3 text-sm text-white outline-none transition-all focus:border-purple-500/50 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                                    />
                                </div>

                                {status === 'loading' && (
                                    <div className="flex flex-col items-center gap-2 py-2">
                                        <div className="text-xs text-purple-400 animate-pulse">{statusText}</div>
                                        <div className="h-1 w-full bg-white/5 rounded-full overflow-hidden">
                                            <div className="h-full bg-purple-500 animate-progress-indefinite" />
                                        </div>
                                    </div>
                                )}

                                {status === 'error' && (
                                    <div className="rounded-lg bg-red-500/10 p-3 text-[12px] text-red-400 border border-red-500/20">
                                        Ошибка: {errorText}
                                    </div>
                                )}
                            </>
                        )}
                    </div>
                </ModalContent>

                <ModalFooter>
                    {status !== 'success' && (
                        <SelectButton
                            onClick={handleStart}
                            className={cn(
                                isLoading && "pointer-events-none opacity-80"
                            )}
                            front={
                                <div className="flex items-center justify-center gap-2">
                                    {isLoading ? (
                                        <>
                                            <svg className="animate-spin h-4 w-4 text-white/70" viewBox="0 0 24 24">
                                                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                                                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                                            </svg>
                                            <span className="text-white/70">Секундочку..</span>
                                        </>
                                    ) : (
                                        "Запустить"
                                    )}
                                </div>
                            }
                            back={
                                <div className="text-purple-300 font-bold">
                                    {isLoading ? "Запускаем..." : "Поехали!"}
                                </div>
                            }
                        />
                    )}
                </ModalFooter>
            </ModalBlock>
        </div>
    );
};