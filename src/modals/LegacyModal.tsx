import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { notify } from "../Notifications";
import { log } from "../Logic";

export const LegacyModal = ({ isOpen }: { isOpen: boolean }) => {
    const [isCleaning, setIsCleaning] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (isOpen) {
            invoke("sync_zapret_files").catch((e) => log("Sync error: " + e));
        }
    }, [isOpen]);

    if (!isOpen) return null;

    const handleAction = async (moveStrategies: boolean) => {
        setIsCleaning(true);
        setError(null);
        try {
            await invoke("run_cleanup", { moveStrategies });
            notify(moveStrategies ? "Данные успешно перенесены!" : "Старая версия удалена.", "success");
            setTimeout(() => window.location.reload(), 1500);
        } catch (e: any) {
            setError(String(e));
            notify("Ошибка при обработке файлов.", "error");
            log("migr err: " + e);
            setIsCleaning(false);
        }
    };

    return (
        <div className="modal-overlay legacy-danger">
            <div className="modal-content legacy-content">
                <div className="modal-body">
                    {isCleaning && !error ? (
                        <div className="loading-state">
                            <div className="spinner">↺</div>
                            <p>Выполняем операцию... ( ¬ ¬)</p>
                        </div>
                    ) : error ? (
                        <div className="error-state">
                            <div className="error-visual">(╥﹏╥)</div>
                            <div className="error-message-box">
                                <p>Не удалось завершить очистку</p>
                                <span className="error-subtext">
                                    Закройте все окна zapret и попробуйте снова.<br />
                                    <i className="error-runtime-text">{error}</i>
                                </span>
                            </div>
                            <button className="retry-btn" onClick={() => handleAction(true)}>
                                Попробовать еще раз
                            </button>
                        </div>
                    ) : (
                        <div className="error-state static-state">
                            <div className="error-visual danger-icon">⚠️</div>
                            <div className="error-message-box">
                                <p>Time-to-clean! ＼(º □ º l|l)/</p>
                                <span className="error-subtext">
                                    Обнаружена папка <b>_up_</b>. Чтобы всё работало корректно, нужно выбрать:
                                    перенести ваши стратегии в безопасное место или просто всё очистить.
                                </span>
                            </div>
                        </div>
                    )}
                </div>

                <div className="modal-footer">
                    <button
                        className="save-modal-btn fix-btn"
                        onClick={() => handleAction(true)}
                        disabled={isCleaning}
                    >
                        {isCleaning ? "Обработка..." : "Перенести стратегии и очистить"}
                    </button>

                    <button
                        className="close-modal-btn"
                        onClick={() => handleAction(false)}
                        disabled={isCleaning}
                    >
                        Удалить старую версию
                    </button>
                </div>
            </div>
        </div>
    );
};