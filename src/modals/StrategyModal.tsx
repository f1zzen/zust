interface Props {
    isOpen: boolean;
    onClose: () => void;
    configs: string[];
    stratName: string;
    onSelect: (name: string) => void;
}

export const StrategyModal = ({ isOpen, onClose, configs, stratName, onSelect }: Props) => {
    if (!isOpen) return null;
    return (
        <div className="modal-overlay" onClick={onClose}>
            <div className="modal-content" onClick={e => e.stopPropagation()}>
                <div className="modal-header">
                    <h3>Выберите стратегию</h3>
                    <span className="strat-count">{configs.length} доступно</span>
                </div>
                <div className="strat-list">
                    {configs.map((cfg) => (
                        <button
                            key={cfg}
                            className={`strat-item ${stratName === cfg ? 'active' : ''}`}
                            onClick={() => onSelect(cfg)}
                        >
                            <div className="strat-info">
                                <span className="strat-name">{cfg.replace('.zapret', '')}</span>
                            </div>
                            {stratName === cfg && <span className="check-icon">✦</span>}
                        </button>
                    ))}
                </div>
                <button className="close-modal-btn" onClick={onClose}>Отмена</button>
            </div>
        </div>
    );
};