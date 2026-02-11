import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModalAnimation } from "./useModalAnimation";
import { log } from "../Logic";

interface ProxyItem {
    address: string;
    ping: number | null | 'loading';
    country?: string;
}

export const ProxyModal = ({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) => {
    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);
    const [proxies, setProxies] = useState<ProxyItem[]>([]);
    const [showNotice, setShowNotice] = useState<string | null>(null);
    const [copied, setCopied] = useState(false);

    const handleProxyClick = async (address: string) => {
        setShowNotice(address);
        setCopied(false);

        try {
            await invoke("open_link", { url: address });
        } catch (e) {
            log("" + e);
        }
    };

    const handleCopy = (text: string) => {
        navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };

    const loadAndPing = async () => {
        try {
            const list: string[] = await invoke("get_proxy_list");
            setProxies(list.map(addr => ({ address: addr, ping: 'loading' })));

            list.forEach(async (addr, index) => {
                try {
                    const result = await invoke<{ ping: number; country_code: string } | null>(
                        "check_proxy_ping",
                        { address: addr }
                    );
                    setProxies(prev => prev.map((p, i) => i === index ? {
                        ...p,
                        ping: result ? result.ping : null,
                        country: result ? result.country_code : undefined
                    } : p));
                } catch (e) {
                    console.error(e);
                }
            });
        } catch (e) { console.error(e); }
    };

    useEffect(() => {
        if (isOpen) loadAndPing();
    }, [isOpen]);

    if (!shouldRender) return null;

    return (
        <div className={`modal-overlay ${isAnimatingOut ? 'closing' : ''}`} onClick={onClose}>
            <div className="modal-content hosts-modal" onClick={e => e.stopPropagation()}>

                <div className="v2-header">
                    <div className="modal-header-left">
                        <div className="modal-title-row">
                            <h3>Telegram | Proxies</h3>
                            <span className="hosts-badge" style={{ background: 'rgba(168, 85, 247, 0.15)', color: '#d8b4fe' }}>
                                {proxies.length} прокси
                            </span>
                        </div>
                        <div className="hosts-update-info">
                            <span className="description-icon">🛫</span>
                            <span>Прокси MTProto для обхода телеграмма!</span>
                        </div>
                    </div>
                    <div className="header-decoration">
                        <div className="glow-dot" style={{ background: '#a855f7', boxShadow: '0 0 10px #a855f7' }}></div>
                    </div>
                </div>

                <div className="modal-body custom-scrollbar" style={{ overflowX: 'hidden' }}>
                    <div className="category-list" style={{ paddingLeft: '8px' }}>
                        {proxies.map((p, i) => (
                            <div key={i} className="category-group" style={{ cursor: 'pointer' }} onClick={() => handleProxyClick(p.address)}>
                                <div className="category-header" style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                                    <div style={{ width: '24px', display: 'flex', justifyContent: 'center', flexShrink: 0 }}>
                                        {p.ping === 'loading' ? (
                                            <span style={{ fontSize: '16px' }}>⏳</span>
                                        ) : p.country && p.country !== "??" ? (
                                            <img
                                                src={`https://purecatamphetamine.github.io/country-flag-icons/3x2/${p.country.toUpperCase()}.svg`}
                                                style={{
                                                    width: '20px',
                                                    height: 'auto',
                                                    borderRadius: '2px',
                                                    filter: 'drop-shadow(0 0 2px rgba(0,0,0,0.2))'
                                                }}
                                                alt={p.country}
                                            />
                                        ) : (
                                            <span style={{ fontSize: '16px' }}>🌐</span>
                                        )}
                                    </div>
                                    <div
                                        className="host-indicator-dot"
                                        style={{
                                            background: p.ping === 'loading' ? '#a855f7' : (p.ping === null ? '#ef4444' : '#22c55e'),
                                            opacity: 1,
                                            flexShrink: 0
                                        }}
                                    />

                                    <span className="category-name" style={{
                                        fontFamily: 'Jetbrains Mono',
                                        fontSize: '13px',
                                        maxWidth: '450px',
                                        overflow: 'hidden',
                                        textOverflow: 'ellipsis',
                                        whiteSpace: 'nowrap',
                                        color: 'rgba(255, 255, 255, 0.8)'
                                    }}>
                                        {p.address}
                                    </span>

                                    <div style={{ marginLeft: 'auto', flexShrink: 0, textAlign: 'right', minWidth: '60px' }}>
                                        {p.ping === 'loading' ? (
                                            <span className="spinner">↻</span>
                                        ) : (
                                            <span style={{
                                                color: p.ping === null ? '#ef4444' : '#22c55e',
                                                fontSize: '11px',
                                                fontWeight: 'bold'
                                            }}>
                                                {p.ping === null ? 'OFFLINE' : `${p.ping}ms`}
                                            </span>
                                        )}
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>

                <div className="modal-footer">
                    <button className="folder-btn" onClick={loadAndPing}>
                        Обновить
                    </button>
                    <button className="close-modal-btn" onClick={onClose}>
                        Закрыть
                    </button>
                </div>

            </div>
            {showNotice && (
                <div style={{
                    position: 'absolute', inset: 0, display: 'flex', alignItems: 'center',
                    justifyContent: 'center', zIndex: 2000, padding: '20px',
                    background: 'rgba(10, 10, 15, 0.8)', backdropFilter: 'blur(4px)',
                    borderRadius: 'inherit', animation: 'fadeIn 0.2s ease-out'
                }}>
                    <div style={{
                        width: '100%', maxWidth: '380px', background: '#1e1e2e',
                        border: '1px solid rgba(168, 85, 247, 0.3)', borderRadius: '16px',
                        padding: '24px', boxShadow: '0 20px 50px rgba(0,0,0,0.5)',
                        textAlign: 'center', position: 'relative'
                    }}>
                        <div style={{ fontSize: '32px', marginBottom: '12px' }}>o( ❛ᴗ❛ )o</div>
                        <h4 style={{ margin: '0 0 10px 0', color: '#fff' }}>Запускаем Telegram</h4>
                        <p style={{ fontSize: '13px', color: 'rgba(255,255,255,0.6)', lineHeight: '1.5', marginBottom: '20px' }}>
                            Если прокси не добавился автоматически, скопируйте ссылку и используйте её вручную.
                        </p>

                        <div
                            onClick={() => handleCopy(showNotice)}
                            style={{
                                background: 'rgba(0,0,0,0.4)', padding: '12px', borderRadius: '8px',
                                fontSize: '11px', color: copied ? '#22c55e' : '#a855f7',
                                wordBreak: 'break-all', cursor: 'pointer', border: '1px solid rgba(168, 85, 247, 0.2)',
                                fontFamily: 'JetBrains Mono', transition: 'all 0.2s', marginBottom: '20px'
                            }}
                        >
                            {copied ? "✓ Ссылка скопирована!" : showNotice}
                        </div>

                        <button
                            onClick={() => setShowNotice(null)}
                            style={{
                                background: 'rgba(168, 85, 247, 0.1)', border: '1px solid #a855f7',
                                color: '#d8b4fe', padding: '8px 24px', borderRadius: '8px',
                                cursor: 'pointer', fontSize: '13px', fontWeight: 'bold', fontFamily: 'Onest'
                            }}
                        >
                            Понятно
                        </button>
                    </div>
                </div>
            )}
        </div>
    );
};