import { useState } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { useModalAnimation } from "./useModalAnimation";

interface Props {
    isOpen: boolean;
    onClose: () => void;
    selectedIpset: string | null;
    onAdd: (ip: string) => void;
}

export const ResolverModal = (p: Props) => {
    const { shouldRender, isAnimatingOut } = useModalAnimation(p.isOpen);
    const [host, setHost] = useState('');
    const [loading, setLoading] = useState(false);

    if (!shouldRender) return null;

    const targetFile = (p.selectedIpset === 'any' || p.selectedIpset === 'none' || !p.selectedIpset)
        ? 'ipset-all.txt'
        : p.selectedIpset;

    const handleResolve = async () => {
        if (!host) return;
        setLoading(true);
        try {
            const [domain, portStr] = host.split(':');
            const port = portStr ? parseInt(portStr) : 25565;
            const ip = await invoke<string>('resolve_host', { host: domain, port });
            p.onAdd(ip);
            setHost('');
            p.onClose();
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className={`modal-overlay ${isAnimatingOut ? 'closing' : ''}`} onClick={p.onClose}>
            <div className="modal-content hosts-modal" onClick={e => e.stopPropagation()} style={{ overflow: 'hidden' }}>

                <div className="v2-header">
                    <div className="modal-header-left">
                        <div className="modal-title-row">
                            <h3>Резолвер</h3>
                            <span className="hosts-badge" style={{ background: 'rgba(168, 85, 247, 0.15)', color: '#d8b4fe' }}>DNS</span>
                        </div>
                        <div className="hosts-update-info">
                            <span className="description-icon">📁</span>
                            Цель: {targetFile}
                        </div>
                    </div>
                    <div className="header-decoration">
                        <div className="glow-dot" style={{ background: '#a855f7', boxShadow: '0 0 10px #a855f7' }}></div>
                    </div>
                </div>

                <div className="modal-body" style={{ overflow: 'hidden' }}>
                    <div className="resolver-body-content" style={{ padding: '24px' }}>
                        <div className="input-group-v2">
                            <input
                                className="resolver-input"
                                placeholder="eu2.catpvp.xyz"
                                value={host}
                                onChange={e => setHost(e.target.value)}
                                onKeyDown={e => e.key === 'Enter' && handleResolve()}
                                disabled={loading}
                                autoFocus
                            />
                        </div>
                        <p className="resolver-desc">
                            Введите буквенный адрес сервес для получения айпи и внедрение его в IPSET-ы. Рекомендуется использовать при проблемах с подключением к серверам.
                        </p>
                    </div>
                </div>

                <div className="modal-footer">
                    <button className="close-modal-btn" onClick={p.onClose} disabled={loading}>
                        Отмена
                    </button>
                    <button
                        className="save-modal-btn resolver-confirm-btn"
                        onClick={handleResolve}
                        disabled={loading || !host.trim()}
                    >
                        {loading ? <span className="spinner">↻</span> : "Применить"}
                    </button>
                </div>

            </div>
        </div>
    );
};