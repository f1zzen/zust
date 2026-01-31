import { openUrl } from '@tauri-apps/plugin-opener';

export const CreditsPage = () => (
    <div className="content credits-container">
        <div className="warning-card">
            <span className="warning-icon">⚠️</span>
            <div className="warning-text">
                <span className="warning-title">ВНИМАНИЕ!!</span>
                <p>Z.UST находится в стадии разработки, и на данный момент может содержать в себе баги. Ваше лучшее решение - либо сообщить об этом на гитхабе, либо написать в личные сообщения ТГК по кнопке ниже.</p>
            </div>
        </div>

        <div className="credits-section">
            <h2 className="section-title">Благодарности</h2>
            <CreditCard title="zapret" author="bol-van" link="https://github.com/bol-van/zapret2" />
            <CreditCard title="zapret-youtube-discord" author="flowseal" link="https://github.com/flowseal/zapret-discord-youtube" />
            <CreditCard title="Icons" author="Lucide" link="https://lucide.dev/" />
            <p className="friends-text" onClick={() => openUrl('https://t.me/fizzeeen')}>...и моим друзьям 🥹</p>
            <p
                className="telegram-btn"
                onClick={() => openUrl('https://t.me/fizzeeen')}
                style={{ cursor: 'pointer' }}
            >
                Подписаться на мой ТГК 👀👀👀
            </p>
        </div>
    </div>
);

interface CreditCardProps {
    title: string;
    author: string;
    link: string;
}

export const CreditCard = ({ title, author, link }: CreditCardProps) => {
    return (
        <div onClick={() => openUrl(link)} className="credit-item" style={{ cursor: 'pointer' }}>
            <span className="credit-name">{title}</span>
            <span className="credit-author">by {author}</span>
        </div>
    );
};