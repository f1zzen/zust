interface Props {
    isOpen: boolean;
    onClose: () => void;
    onPick: () => void;
}

export const ConvertModal = ({ isOpen, onClose, onPick }: Props) => {
    if (!isOpen) return null;
    return (
        <div className="modal-overlay blur" onClick={onClose}>
            <div className="modal-content glass-morphism" onClick={e => e.stopPropagation()}>
                <div className="modal-header-fancy"><h3>Конвертатор</h3></div>
                <div className="modal-body">
                    <p className="description-text">
                        Конвертация <span className="highlight">.bat</span> в <span className="highlight">.zapret</span> одним нажатием.
                    </p>
                    <div className="drop-zone" onClick={onPick}>
                        <div className="lava-lamp-container">
                            <div className="lava-blob blob-1"></div>
                            <div className="lava-blob blob-2"></div>
                            <div className="lava-blob blob-3"></div>
                        </div>
                        <div className="drop-content">
                            <svg className="upload-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                                <path d="M12 17V3m0 0l-4 4m4-4l4 4m-9 10a4 4 0 00-4 4h18a4 4 0 00-4-4" strokeLinecap="round" strokeLinejoin="round" />
                            </svg>
                            <span className="drop-text">ВЫБРАТЬ</span>
                            <span className="drop-subtext">Только .bat файлы!</span>
                        </div>
                        {/* SVG Фильтр остается здесь, внутри компонента */}
                        <svg style={{ position: 'absolute', width: 0, height: 0 }}>
                            <filter id="lava-goo">
                                <feGaussianBlur in="SourceGraphic" stdDeviation="10" result="blur" />
                                <feColorMatrix in="blur" mode="matrix" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 18 -7" result="lava-goo" />
                                <feBlend in="SourceGraphic" in2="lava-goo" />
                            </filter>
                        </svg>
                    </div>
                </div>
                <button className="cancel-link" onClick={onClose}>Отмена</button>
            </div>
        </div>
    );
};