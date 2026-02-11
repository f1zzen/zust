import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export const EditorPage = () => {
    const [loading, setLoading] = useState(true);
    const [files, setFiles] = useState<string[]>([]);
    const [selectedFile, setSelectedFile] = useState<string | null>(null);
    const [content, setContent] = useState("");

    useEffect(() => { loadFiles(); }, []);

    async function loadFiles() {
        setLoading(true);
        try {
            const result = await invoke<string[]>('get_list_files');
            setFiles(result);
        } catch (e) { console.error(e); }
        finally { setLoading(false); }
    }

    async function openFile(name: string) {
        setLoading(true);
        try {
            const text = await invoke<string>('read_file', { name });
            setContent(text);
            setSelectedFile(name);
        } catch (e) { console.error(e); }
        finally { setLoading(false); }
    }

    async function saveFile() {
        if (!selectedFile) return;
        try {
            await invoke('save_file', { name: selectedFile, content });
            setSelectedFile(null);
            loadFiles();
        } catch (e) { console.error(e); }
    }

    if (selectedFile) {
        return (
            <div className="content">
                <button className="back-button" onClick={() => setSelectedFile(null)}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                        <line x1="19" y1="12" x2="5" y2="12"></line>
                        <polyline points="12 19 5 12 12 5"></polyline>
                    </svg>
                    Назад к списку
                </button>

                <div className="strategy-header">
                    <div className="strat-title-row">
                        <span className="strat-value">{selectedFile}</span>
                    </div>
                </div>

                <textarea
                    className="editor-textarea"
                    value={content}
                    onChange={(e) => setContent(e.target.value)}
                    spellCheck={false}
                />

                <div className="action-buttons">
                    <button className="save-modal-btn" onClick={saveFile}>
                        СОХРАНИТЬ ИЗМЕНЕНИЯ
                    </button>
                </div>
            </div>
        );
    }

    return (
        <div className="content">
            <div className="strategy-header">
                <span className="strat-label">FILES</span>
                <div className="strat-title-row">
                    <span className="strat-value">Редактор списков</span>
                </div>
            </div>
            <div className="editor-container">
                {loading ? (
                    <div className="loader-box">
                        <div className="spinner"></div>
                        <span className="loading-text">СКАНИРОВАНИЕ...</span>
                    </div>
                ) : (
                    <div className="file-grid">
                        {files.map((file, i) => (
                            <button key={file} className="file-card" onClick={() => openFile(file)} style={{ animationDelay: `${i * 0.1}s` }}>
                                <div className="file-icon">📄</div>
                                <span className="file-name">{file}</span>
                            </button>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
};