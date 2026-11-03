import { Modal, CloseButton } from "./Modal";
import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { listen } from "@tauri-apps/api/event";
import { InfoHeader, ModalContent, ModalFooter, ModalHeader, SelectButton, WarningHeader } from "@/Buttons";
import { notify } from "@/Notifications";

interface RepairModalProps {
    stratName: string;
}

export const RepairZapret4TorModal = ({ stratName }: RepairModalProps) => {
    const [isOpen, setIsOpen] = useState(false);
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
            await api.repairZapretForTor(stratName);
            notify("Zapret успешно настроен для Tor Project", "success");
            setIsOpen(false);
        } catch (e) {
            notify(`Ошибка: ${e}`, "error");
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <Modal isOpen={isOpen} onClose={() => setIsOpen(false)} locked={isLoading}>
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
                    <CloseButton onClick={() => setIsOpen(false)}>Закрыть</CloseButton>
                </ModalFooter>
            </Modal>
    );
};