import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';

export const ExtraPage = ({
    logs,
    logStart
}: any) => (
    <div className="content credits-container">
        <div className="credits-main-scroll">
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
                <p className="friends-text">...и моим друзьям 🥹</p>
            </div>
            <button className="telegram-btn" onClick={async () => {
                await invoke("open_link", { url: "https://t.me/zdotust" });
            }}>
                <svg className="telegram-icon" width="24px" height="24px" viewBox="0 0 192 192" xmlns="http://www.w3.org/2000/svg" fill="none"><path stroke="currentColor" strokeWidth="10" d="M23.073 88.132s65.458-26.782 88.16-36.212c8.702-3.772 38.215-15.843 38.215-15.843s13.621-5.28 12.486 7.544c-.379 5.281-3.406 23.764-6.433 43.756-4.54 28.291-9.459 59.221-9.459 59.221s-.756 8.676-7.188 10.185c-6.433 1.509-17.027-5.281-18.919-6.79-1.513-1.132-28.377-18.106-38.214-26.404-2.649-2.263-5.676-6.79.378-12.071 13.621-12.447 29.891-27.913 39.728-37.72 4.54-4.527 9.081-15.089-9.837-2.264-26.864 18.483-53.35 35.835-53.35 35.835s-6.053 3.772-17.404.377c-11.351-3.395-24.594-7.921-24.594-7.921s-9.08-5.659 6.433-11.693Z" /></svg>
                <span className="telegram-text">Подписаться на канал</span>
            </button>
        </div>

        <div className={`log-container ${logs.length > 0 ? 'visible' : 'hidden'}`}>
            <div className="log-header">Логи</div>
            <div className="log-content">
                {[...logs].reverse().map((log, i) => {
                    const match = log.match(/^(\[.*?\])\s*(.*)/);
                    const time = match ? match[1] : '';
                    const msg = match ? match[2] : log;

                    return (
                        <div key={i} className="log-line">
                            {time && <span className="log-timestamp">{time}</span>}
                            <span className="log-message">{msg}</span>
                        </div>
                    );
                })}
                <div ref={logStart} />
            </div>
        </div>
    </div>
);

interface CreditCardProps {
    title: string;
    author: string;
    link: string;
}

export const CreditCard = ({ title, author, link }: CreditCardProps) => (
    <div onClick={() => openUrl(link)} className="credit-item">
        <span className="credit-name">{title}</span>
        <span className="credit-author">by {author}</span>
    </div>
);