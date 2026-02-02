import { invoke } from "@tauri-apps/api/core";
import { useModalAnimation } from "./useModalAnimation";

interface Props {
    isOpen: boolean;
    onClose: () => void;
    selectedIpset: string | null;
    ipsetView: 'main' | 'custom';
    setIpsetView: (v: 'main' | 'custom') => void;
    customIpsetFiles: string[];
    onModeChange: (mode: 'Any' | 'None' | 'Custom', file?: string) => void;
    loadCustom: () => void;
    hoveredDesc: string | null;
    setHoveredDesc: (s: string | null) => void;
}

const DESCRIPTIONS: Record<string, string> = {
    'Any': 'Использовать стандартный список ipset-all.txt для обхода',
    'None': 'Не использовать списки фильтрации. ZAPRET будет игнорировать IP-адреса.',
    'ipset-all.txt': 'Будет использован пользовательский набор из zapret/lists/ipset-all.txt',
    'custom': 'Фильтрация по вашему индивидуальному списку адресов.'
};

export const IpsetModal = (p: Props) => {
    const { shouldRender, isAnimatingOut } = useModalAnimation(p.isOpen);
    if (!shouldRender) return null;

    return (
        <div className={`modal-overlay ${isAnimatingOut ? 'closing' : ''}`} onClick={p.onClose}>
            <div className="modal-content ipset-modal-content" onClick={e => e.stopPropagation()}>
                <div className="modal-header">
                    <h3>{p.ipsetView === 'main' ? 'Режим фильтрации' : 'Пользовательские списки'}</h3>
                </div>

                <div className="ipset-container">
                    {p.ipsetView === 'main' ? (
                        <div className="ipset-modal-grid">
                            <button
                                className={`ipset-item ${p.selectedIpset === 'any' ? 'active' : ''}`}
                                onClick={() => { p.onModeChange('Any'); p.onClose(); }}
                                onMouseEnter={() => p.setHoveredDesc(DESCRIPTIONS['Any'])}
                                onMouseLeave={() => p.setHoveredDesc(null)}
                            >
                                <span className="item-label">🌐 Весь трафик (Any)</span>
                            </button>

                            <button
                                className={`ipset-item ${p.selectedIpset === 'none' ? 'active' : ''}`}
                                onClick={() => { p.onModeChange('None'); p.onClose(); }}
                                onMouseEnter={() => p.setHoveredDesc(DESCRIPTIONS['None'])}
                                onMouseLeave={() => p.setHoveredDesc(null)}
                            >
                                <span className="item-label">🚫 Отключить (None)</span>
                            </button>

                            <button
                                className="ipset-item ipset-loaded-btn"
                                onClick={() => { p.loadCustom(); p.setIpsetView('custom'); }}
                                onMouseEnter={() => p.setHoveredDesc(DESCRIPTIONS['custom'])}
                                onMouseLeave={() => p.setHoveredDesc(null)}
                            >
                                <span className="item-label">📄 Пользовательский (Loaded)</span>
                            </button>
                        </div>
                    ) : (
                        <div className="ipset-file-view">
                            <div className="ipset-controls">
                                <button className="back-btn" onClick={() => p.setIpsetView('main')}>← Назад</button>
                                <button className="folder-btn" onClick={() => invoke('open_ipset_dir')}>Папка 📂</button>
                            </div>

                            <div className="ipset-file-list">
                                <button
                                    className={`ipset-item wide ${p.selectedIpset === 'ipset-all.txt' ? 'active' : ''}`}
                                    onClick={() => { p.onModeChange('Custom', 'ipset-all.txt'); p.onClose(); }}
                                    onMouseEnter={() => p.setHoveredDesc(DESCRIPTIONS['ipset-all.txt'])}
                                    onMouseLeave={() => p.setHoveredDesc(null)}
                                >
                                    <span className="item-label">⭐ ipset-all.txt</span>
                                </button>
                                {p.customIpsetFiles.map(file => (
                                    <button
                                        key={file}
                                        className={`ipset-item wide ${p.selectedIpset === file ? 'active' : ''}`}
                                        onClick={() => { p.onModeChange('Custom', file); p.onClose(); }}
                                        onMouseLeave={() => p.setHoveredDesc(null)}
                                        onMouseEnter={() => p.setHoveredDesc(DESCRIPTIONS['custom'])}
                                    >
                                        <span className="item-label">📄 {file.replace('.txt', '').replace(/_/g, ' ')}</span>
                                    </button>
                                ))}
                            </div>
                        </div>
                    )}
                </div>

                <div className="modal-footer-desc">
                    <div className={`desc-content ${p.hoveredDesc ? 'visible' : ''}`}>
                        <span className="desc-icon">ℹ</span>
                        <p className="desc-text">{p.hoveredDesc || 'Выберите режим'}</p>
                    </div>
                </div>

                <button className="close-modal-btn" style={{ marginTop: '10px' }} onClick={p.onClose}>Закрыть</button>
            </div>
        </div>
    );
};