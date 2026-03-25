import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useModalAnimation } from "./useModalAnimation";
import { InfoHeader, ModalBlock, ModalContent, ModalFooter, ModalHeader, SelectButton, WarningHeader } from "@/Buttons";
import { cn } from "@/lib/utils";
import React from "react";
import { notify } from "@/Notifications";

interface RepairModalProps {
    stratName: string;
}

export const RepairZapret4TorModal = ({ stratName }: RepairModalProps) => {
    const [isOpen, setIsOpen] = useState(false);
    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);
    const [isLoading, setIsLoading] = useState(false);

    useEffect(() => {
        const initListener = async () => {
            const unlisten = await listen("zapret_repair", () => {
                setIsOpen(true);
            });
            return unlisten;
        };

        const listenerPromise = initListener();
        return () => {
            listenerPromise.then(unlisten => unlisten());
        };
    }, []);

    const handleConfirm = async () => {
        setIsLoading(true);
        try {
            await invoke("zapret_4_tor", { stratName: stratName });
            notify("Zapret успешно настроен для Tor Project", "success");
            setIsOpen(false);
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
            onClick={() => !isLoading && setIsOpen(false)}
        >
            <ModalBlock onClick={(e: React.MouseEvent) => e.stopPropagation()}>
                <ModalHeader
                    title="Внимание!"
                    status="(⌒_⌒;)"
                    icon="🚫"
                    description="Обнаружены проблемы с получением мостов.."
                />

                <ModalContent>
                    <div className="space-y-4 py-2">
                        <InfoHeader
                            description='Роскомнадзор на основании решения суда ограничил доступ к ресурсу Tor Project. "На основании судебного решения ресурс https://www.torproject.org внесен в Единый реестр запрещенной информации. <...> На сегодняшний день доступ к ресурсу ограничен",'
                            more="Информация взята с российского СМИ - ТАСС*"
                        />
                        {isLoading && (
                            <div className="flex flex-col items-center gap-2 py-2">
                                <div className="text-xs text-purple-400 animate-pulse">Применение конфигурации...</div>
                                <div className="h-1 w-full bg-white/5 rounded-full overflow-hidden">
                                    <div className="h-full bg-purple-500 animate-progress-indefinite" />
                                </div>
                            </div>
                        )}
                        <WarningHeader title="Сбор мостов для прокси не удался." description="Для обхода блокировки серверов Tor Project модифицируйте Z.UST (точнее, zapret) кнопкой ниже. Заст автоматически добавит torproject.org в ваш список (list-general.txt), чтобы соединение установилось. После активации попробуйте снова получить мосты." />
                    </div>
                </ModalContent>

                <ModalFooter>
                    <SelectButton onClick={handleConfirm} front="Применить" back="Настройки" />
                    <button
                        className="rounded-lg bg-white/5 px-4 py-1 font-['Onest'] text-[13px] font-bold text-white/60 transition-all hover:bg-white/10 hover:text-white active:scale-95"
                        onClick={() => setIsOpen(false)}
                    >
                        Закрыть
                    </button>
                </ModalFooter>
            </ModalBlock>
        </div>
    );
};