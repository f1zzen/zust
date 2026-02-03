import { useModalAnimation } from "./useModalAnimation";
import { invoke } from "@tauri-apps/api/core";

interface Props {
    isOpen: boolean;
    onClose: () => void;
    configs: string[];
    stratName: string;
    onSelect: (name: string) => void;
}

export const StrategyModal = ({ isOpen, onClose, configs, stratName, onSelect }: Props) => {
    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);
    if (!shouldRender) return null;
    return (
        <div className={`modal-overlay ${isAnimatingOut ? 'closing' : ''}`} onClick={onClose}>
            <div className="modal-content hosts-modal" onClick={e => e.stopPropagation()}>
                <div className="v2-header">
                    <div className="modal-header-left">
                        <div className="modal-title-row">
                            <h3>Стратегии</h3>
                            <span className="hosts-badge">{configs.length} доступно</span>
                        </div>
                        <div className="hosts-update-info">
                            <span className="description-icon">📂</span>
                            <span>Конфигурация запрета</span>
                        </div>
                    </div>
                    <div className="header-decoration">
                        <div className="glow-dot"></div>
                    </div>
                </div>

                <div className="modal-body">
                    <div className="category-list">
                        {configs.map((cfg) => (
                            <div key={cfg} className={`category-group ${stratName === cfg ? 'active-strat' : ''}`}>
                                <div className="category-header" onClick={() => onSelect(cfg)}>
                                    <div className="host-indicator-dot" style={{ opacity: stratName === cfg ? 1 : 0.3 }} />
                                    <span className="category-name">{cfg.replace('.zapret', '')}</span>
                                    {stratName === cfg && <span className="hosts-badge" style={{ marginLeft: 'auto' }}>Активно</span>}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>

                <div className="modal-footer">
                    <button className="folder-btn" onClick={() => invoke('open_strats_dir')}>
                        Открыть папку
                    </button>
                    <button className="close-modal-btn" onClick={onClose}>
                        Закрыть
                    </button>
                </div>
            </div>
        </div>
    );
};