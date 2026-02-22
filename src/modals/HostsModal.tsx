import { useEffect, useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { notify } from "../Notifications"
import { log } from "../Logic";

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
    const [expanded, setExpanded] = useState<string[]>([]);
    const [selected, setSelected] = useState<string[]>([]);
    const [saving, setSaving] = useState(false);
    const [lastUpdate, setLastUpdate] = useState<string>("");

    const [shouldRender, setShouldRender] = useState(isOpen);
    const [isAnimatingOut, setIsAnimatingOut] = useState(false);

    const TIMEOUT = 10 * 1000;
    const LIMIT_DOMAINS = 50;

    const handleClear = async () => {
        setSaving(true);
        try {
            await invoke("save_hosts_selection", { selectedLines: [] });
            notify("Записи успешно удалены!", "success");
            onClose();
        } catch (e) {
            notify("Ошибка при очистке hosts.", "error");
            log("hosts err " + e);
        } finally {
            setSaving(false);
        }
    };

    const loadHosts = async () => {
        setLoading(true);
        setError(null);
        try {
            const fetchPromise = invoke<{ date: string, categories: Record<string, string[]> }>("get_hosts_data");
            const timeoutPromise = new Promise((_, reject) =>
                setTimeout(() => reject(new Error("timeout")), TIMEOUT)
            );
            const result = await (Promise.race([fetchPromise, timeoutPromise]) as Promise<{ date: string, categories: Record<string, string[]> }>);
            setData(result.categories);
            setLastUpdate(result.date);
        } catch (e: any) {
            console.error(e);
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

    const sortedKeys = useMemo(() => {
        return Object.keys(data).sort((a, b) => {
            if (a === BASIC) return -1;
            if (b === BASIC) return 1;
            if (a === "Блокировка") return 1;
            if (b === "Блокировка") return -1;
            return a.localeCompare(b);
        });
    }, [data]);

    if (!shouldRender) return null;

    const toggleExpand = (name: string) => {
        setExpanded(prev => prev.includes(name) ? prev.filter(n => n !== name) : [...prev, name]);
    };

    const selectAll = () => {
        setSelected(Object.keys(data));
    };

    const deselectAll = () => {
        setSelected([]);
    };

    const handleSave = async () => {
        setSaving(true);
        try {
            const malwLines: string[] = [];
            const flowLines: string[] = [];

            selected.forEach(cat => {
                const lines = data[cat];
                if (cat === "Discord & Telegram") {
                    flowLines.push(`# ${cat}`, ...lines);
                } else {
                    malwLines.push(`# ${cat}`, ...lines);
                }
            });
            let finalString = "";
            if (malwLines.length > 0) {
                finalString += `### dns.malw.link: hosts file\n${malwLines.join('\n')}\n### dns.malw.link: end hosts file\n\n`;
            }
            if (flowLines.length > 0) {
                finalString += `### flowseal: hosts file\n${flowLines.join('\n')}\n### flowseal: end hosts file\n`;
            }

            await invoke("save_hosts_selection", { selectedLines: finalString });
            notify("Метод hosts успешно применен!", "success");
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
                <div className="v2-header">
                    <div className="modal-header-left">
                        <div className="modal-title-row">
                            <h3>Настройка</h3>
                            {!loading && !error && (
                                <span className="hosts-badge">{Object.keys(data).length} групп</span>
                            )}
                        </div>
                        <div className="hosts-update-info">
                            <span className="calendar-icon">📅</span>
                            <span className="calendar-text">Обновлено: {lastUpdate}</span>
                        </div>
                    </div>
                    <div className="header-decoration">
                        <div className="glow-dot"></div>
                    </div>
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
                        <div className="hosts-scroll-area">
                            <div className="category-list">
                                {sortedKeys.map((name) => {
                                    const lines = data[name];
                                    return (
                                        <div key={name} className="category-group">
                                            <div
                                                className={`category-header ${expanded.includes(name) ? 'active' : ''}`}
                                                onClick={() => toggleExpand(name)}
                                            >
                                                <div className="checkbox-wrapper" onClick={e => e.stopPropagation()}>
                                                    <label className="custom-checkbox">
                                                        <input
                                                            type="checkbox"
                                                            checked={selected.includes(name)}
                                                            onChange={e => setSelected(prev => e.target.checked ? [...prev, name] : prev.filter(n => n !== name))}
                                                        />
                                                        <span className="checkmark"></span>
                                                    </label>
                                                </div>

                                                <span className="category-name">{name}</span>

                                                <div className={`arrow-icon ${expanded.includes(name) ? 'rotated' : ''}`}>
                                                    <svg width="12" height="8" viewBox="0 0 12 8" fill="none">
                                                        <path d="M1 1.5L6 6.5L11 1.5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                                                    </svg>
                                                </div>
                                            </div>

                                            <div className={`category-content-wrapper ${expanded.includes(name) ? 'is-open' : ''}`}>
                                                <div className="category-content-inner">
                                                    <div className="category-lines">
                                                        {lines.slice(0, LIMIT_DOMAINS).map((line, i) => {
                                                            const [ip, ...domainParts] = line.trim().split(/\s+/);
                                                            return (
                                                                <div key={i} className="host-row" style={{ animationDelay: `${i * 15}ms` }}>
                                                                    <div className="host-indicator-dot" />
                                                                    <span className="host-ip">{ip}</span>
                                                                    <span className="host-domain">{domainParts.join(' ')}</span>
                                                                </div>
                                                            );
                                                        })}
                                                        {lines.length > LIMIT_DOMAINS && (
                                                            <div className="host-more-info">
                                                                Показано {LIMIT_DOMAINS} из {lines.length} записей.
                                                                <span className="host-limited-label">Лимит</span>
                                                            </div>
                                                        )}
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>
                            {!loading && !error && (
                                <div className="clear-hosts-container">
                                    <div className="selection-controls" style={{ paddingLeft: '10px' }}>
                                        <button className="select-strat-btn" onClick={selectAll}>Выбрать всё</button>
                                        <button className="select-strat-btn" style={{ background: 'rgba(255,255,255,0.05)', borderColor: 'rgba(255,255,255,0.1)', color: '#888' }} onClick={deselectAll}>Убрать всё</button>
                                    </div>
                                    <button
                                        className="clear-hosts-btn"
                                        onClick={handleClear}
                                        disabled={saving}
                                    >
                                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                                            <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M10 11v6M14 11v6" />
                                        </svg>
                                        Очистить записи
                                    </button>
                                    <p className="clear-hint">
                                        Не будут очищены остальные записи, кроме dns.malw.link
                                    </p>
                                </div>
                            )}
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