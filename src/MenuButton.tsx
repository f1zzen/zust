export const MenuButton = ({ title, subtitle, icon, onClick, variant }: any) => (
    <button className={`wide-btn ${variant}-variant`} onClick={onClick}>
        <div className="btn-content">
            <span className="btn-icon">{icon}</span>
            <div className="btn-text">
                <span className="btn-title">{title}</span>
                <span className="btn-subtitle">{subtitle}</span>
            </div>
        </div>
        <span className="btn-arrow">→</span>
    </button>
);