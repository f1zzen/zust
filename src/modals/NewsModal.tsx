import { useState, useEffect } from "react";
import ReactMarkdown from 'react-markdown';
import { invoke } from "@tauri-apps/api/core";
import { log } from "../Logic";

export const NewsModal = ({
    updateAvailable,
    onClose
}: {
    updateAvailable: string | null,
    onClose: () => void
}) => {
    const [news, setNews] = useState<{ title: string; content: string } | null>(null);
    const [isVisible, setIsVisible] = useState(false);

    useEffect(() => {
        const fetchNews = async () => {
            try {
                const rawUrl = "https://gist.githubusercontent.com/f1zzen/92a5742eb95e6e1922b268e9dd586f45/raw/warning";
                const response = await fetch(`${rawUrl}?t=${Date.now()}`, { cache: 'no-store' });
                if (!response.ok) return;

                const text = await response.text();
                const lines = text.split('\n');
                const title = lines[0].replace(/^#\s*/, '').trim();
                const content = lines.slice(1).join('\n').trim();

                if (title || content) {
                    const lastSeenHash = localStorage.getItem("last_seen_news_hash");
                    const currentHash = title + content;
                    setNews({ title, content });
                    if (updateAvailable || lastSeenHash !== currentHash) {
                        setIsVisible(true);
                    }
                }
            } catch (e) { log("" + e); }
        };
        fetchNews();
    }, [updateAvailable]);
    const handleInternalClose = () => {
        if (news) {
            localStorage.setItem("last_seen_news_hash", news.title + news.content);
        }

        setIsVisible(false);
        onClose();
    };
    if (!isVisible || !news) return null;
    return (
        <div className="modal-overlay legacy-danger" style={{ zIndex: 9999 }}>
            <div className="modal-content legacy-content" style={{
                maxWidth: '460px',
                border: updateAvailable ? '1px solid rgba(59, 130, 246, 0.5)' : '1px solid rgba(168, 85, 247, 0.4)',
                background: '#11111b'
            }}>
                <div className="modal-body" style={{ textAlign: 'left', padding: '20px' }}>
                    <div style={{
                        background: updateAvailable ? 'rgba(59, 130, 246, 0.12)' : 'rgba(168, 85, 247, 0.12)',
                        color: updateAvailable ? '#60a5fa' : '#c084fc',
                        width: 'fit-content', padding: '2px 8px', borderRadius: '6px',
                        fontSize: '10px', fontWeight: '700', marginBottom: '15px',
                        textTransform: 'uppercase', letterSpacing: '0.04em',
                        border: `1px solid ${updateAvailable ? 'rgba(59, 130, 246, 0.2)' : 'rgba(168, 85, 247, 0.15)'}`
                    }}>
                        {updateAvailable ? `Доступна v${updateAvailable}` : 'Объявление'}
                    </div>

                    <h2 style={{ fontSize: '24px', color: '#fff', marginBottom: '18px', fontFamily: 'JetBrains Mono, monospace', lineHeight: '1.2' }}>
                        {news.title}
                    </h2>

                    <div className="markdown-render custom-scrollbar" style={{ fontSize: '14px', color: 'rgba(255,255,255,0.7)', lineHeight: '1.6', maxHeight: '300px', overflowY: 'auto', paddingRight: '10px' }}>
                        <ReactMarkdown components={{
                            a: ({ ...props }) => (
                                <a {...props} onClick={(e) => { e.preventDefault(); if (props.href) invoke("open_link", { url: props.href }); }}
                                    style={{ color: '#c084fc', cursor: 'pointer', textDecoration: 'underline' }} />
                            ),
                            strong: ({ children }) => <strong style={{ color: '#d8b4fe' }}>{children}</strong>,
                        }}>
                            {news.content}
                        </ReactMarkdown>
                    </div>
                </div>

                <div className="modal-footer" style={{ borderTop: '1px solid rgba(255,255,255,0.05)', padding: '20px 28px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
                    {updateAvailable && (
                        <button
                            className="save-modal-btn"
                            onClick={() => invoke("open_link", { url: "https://github.com/f1zzen/zust/releases" })}
                            style={{ background: 'rgba(59, 130, 246, 0.15)', color: '#60a5fa', borderColor: 'rgba(59, 130, 246, 0.3)' }}
                        >
                            Скачать обновление
                        </button>
                    )}
                    <button className="save-modal-btn" onClick={handleInternalClose} style={{ width: '100%' }}>
                        Понятно
                    </button>
                </div>
            </div>
        </div>
    );
};