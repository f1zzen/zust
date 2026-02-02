import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { notify } from "../Notifications"

interface Props {
    isOpen: boolean;
    onClose: () => void;
}

export const HostsModal = ({ isOpen, onClose }: Props) => {
    const ERR_TIMEOUT = "ERR: (Проверьте интернет!)";
    const ERR_READING = "ERR: (Не удалось прочитать данные?)"
    const BASIC = "Базовая";
    const [data, setData] = useState<Record<string, string[]>>({});
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [expanded, setExpanded] = useState<string[]>([BASIC]);
    const [selected, setSelected] = useState<string[]>([]);
    const [saving, setSaving] = useState(false);

    const [shouldRender, setShouldRender] = useState(isOpen);
    const [isAnimatingOut, setIsAnimatingOut] = useState(false);

    const TIMEOUT = 10 * 1000;
    const LIMIT_DOMAINS = 50;

    const handleClear = async () => {
        if (!confirm("Вы уверены, что хотите полностью очистить записи hosts?")) return;

        setSaving(true);
        try {
            await invoke("save_hosts_selection", { selectedLines: [] });
            notify("Записи успешно удалены!", "success");
            onClose();
        } catch (e) {
            notify("Ошибка при очистке hosts.", "error");
        } finally {
            setSaving(false);
        }
    };

    const loadHosts = async () => {
        setLoading(true);
        setError(null);
        try {
            const fetchPromise = invoke<Record<string, string[]>>("get_hosts_data");
            const timeoutPromise = new Promise((_, reject) =>
                setTimeout(() => reject(new Error("timeout")), TIMEOUT)
            );

            const result = await (Promise.race([fetchPromise, timeoutPromise]) as Promise<Record<string, string[]>>);
            setData(result);
            setSelected(Object.keys(result));
        } catch (e: any) {
            setError(e.message === "timeout" ? ERR_TIMEOUT : ERR_READING);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        if (isOpen) {
            setShouldRender(true);
            setIsAnimatingOut(false);
        } else if (shouldRender) {
            setIsAnimatingOut(true);
            const timer = setTimeout(() => {
                setShouldRender(false);
                setIsAnimatingOut(false);
            }, 300);
            return () => clearTimeout(timer);
        }
    }, [isOpen]);

    useEffect(() => {
        if (isOpen && Object.keys(data).length === 0) {
            loadHosts();
        }
    }, [isOpen]);

    if (!shouldRender) return null;

    const toggleExpand = (name: string) => {
        setExpanded(prev => prev.includes(name) ? prev.filter(n => n !== name) : [...prev, name]);
    };

    const handleSave = async () => {
        setSaving(true);
        try {
            const formattedLines = selected.flatMap(cat => [
                `# ${cat}`,
                ...data[cat],
                ""
            ]);
            await invoke("save_hosts_selection", { selectedLines: formattedLines });
            notify("Метод hosts успешно применен!", "success");
            notify("Для некоторых приложений потребуется перезагрузка.", "success");
            onClose();
        } catch (e) {
            notify("Произошла ошибка во время записи hosts.", "error");
        } finally {
            setSaving(false);
        }
    };

    return (
        <div
            className={`modal-overlay ${isAnimatingOut ? 'closing' : ''}`}
            onClick={onClose}
        >
            <div
                className="modal-content hosts-modal"
                onClick={e => e.stopPropagation()}
            >
                <div className="modal-header">
                    <h3>Настройка</h3>
                    {!loading && !error && <span className="strat-count">{Object.keys(data).length} категорий</span>}
                </div>

                <div className="modal-body">
                    {loading && (
                        <div className="loading-state">
                            <div className="spinner">↺</div>
                            <p>Подождите, мы обновляем список.. (¬ ¬ )</p>
                        </div>
                    )}

                    {error && (
                        <div className="error-state">
                            <div className="error-visual">(╥﹏╥)</div>
                            <div className="error-message-box">
                                <p>{error}</p>
                                <span className="error-subtext">Произошла ошибка, и вероятнее всего на вашей стороне. Попробуйте переподключиться по кнопке ниже.. </span>
                            </div>
                            <button className="retry-btn" onClick={loadHosts}>
                                Переподключиться
                            </button>
                        </div>
                    )}

                    {!loading && !error && (
                        <div className="category-list">
                            {Object.entries(data)
                                .sort(([nameA], [nameB]) => {
                                    if (nameA === BASIC) return -1;
                                    if (nameB === BASIC) return 1;
                                    if (nameA === "Блокировка") return 1;
                                    if (nameB === "Блокировка") return -1;
                                    return nameA.localeCompare(nameB);
                                })
                                .map(([name, lines]) => (
                                    <div key={name} className="category-group">
                                        <div className="category-header" onClick={() => toggleExpand(name)}>
                                            <div className="checkbox-container" onClick={(e) => e.stopPropagation()}>
                                                <input
                                                    type="checkbox"
                                                    id={`check-${name}`}
                                                    checked={selected.includes(name)}
                                                    onChange={(e) => {
                                                        setSelected(prev => e.target.checked ? [...prev, name] : prev.filter(n => n !== name));
                                                    }}
                                                />
                                                <label htmlFor={`check-${name}`}></label>
                                            </div>

                                            <span className="category-name">{name}</span>
                                            <span className={`arrow-icon ${expanded.includes(name) ? 'rotated' : ''}`}>▸</span>
                                        </div>

                                        <div className={`category-lines-wrapper ${expanded.includes(name) ? 'expanded' : ''}`}>
                                            <div className="category-lines">
                                                {lines.slice(0, LIMIT_DOMAINS).map((line, i) => (
                                                    <div key={`${name}-${i}`} className="host-row">
                                                        <div className="host-dot"></div>
                                                        <span className="host-text">{line}</span>
                                                    </div>
                                                ))}

                                                {lines.length > LIMIT_DOMAINS && (
                                                    <div className="host-more-info">
                                                        Показано {LIMIT_DOMAINS} из {lines.length} записей.
                                                        <span className="host-limited-label">Лимит</span>
                                                    </div>
                                                )}
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            <div className="clear-hosts-container">
                                <button
                                    className="clear-hosts-btn"
                                    onClick={handleClear}
                                    disabled={saving}
                                >
                                    <span className="icon">🗑</span>
                                    Очистить записи hosts
                                </button>
                                <p className="clear-hint">Это удалит только блок dns.malw.link, системные записи не пострадают.</p>
                            </div>
                        </div>
                    )}
                </div>

                <div className="modal-footer">
                    <button className="close-modal-btn" onClick={onClose} disabled={saving}>
                        Отмена
                    </button>
                    <button
                        className="save-modal-btn"
                        onClick={handleSave}
                        disabled={loading || !!error || saving}
                    >
                        {saving ? "Сохранение..." : "Применить"}
                    </button>
                </div>
            </div>
        </div >
    );
};